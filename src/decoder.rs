use aes::Aes128;
use anyhow::{Result, bail};
use ecb::Decryptor;
use ecb::cipher::block_padding::Pkcs7;
use ecb::cipher::{BlockDecryptMut, KeyInit};
use serde_json::Value;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use base64ct::Encoding;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, BufWriter};
use tracing::{debug, error, instrument};

type Aes128EcbDec = Decryptor<Aes128>;
type KeyBox = [usize; 256];
// 常量定义
const CORE_KEY: [u8; 16] = hex_literal::hex!("687A4852416D736F356B496E62617857");
const META_KEY: [u8; 16] = hex_literal::hex!("2331346C6A6B5F215C5D2630553C2728");
const NCM_HEADER: [u8; 8] = hex_literal::hex!("4354454e4644414d");
const BUFFER_SIZE: usize = 32 * 1024; // 64KB
const KEY_XOR_VALUE: u8 = 0x64;
const META_XOR_VALUE: u8 = 0x63;

// 全局静态变量，使用 LazyLock 确保线程安全
static CORE_CIPHER: LazyLock<Aes128EcbDec> = LazyLock::new(|| Aes128EcbDec::new(&CORE_KEY.into()));
static META_CIPHER: LazyLock<Aes128EcbDec> = LazyLock::new(|| Aes128EcbDec::new(&META_KEY.into()));
static KEY_BOX: LazyLock<KeyBox> = LazyLock::new(|| {
    let mut box_array = [0; 256];
    for (i, item) in box_array.iter_mut().enumerate() {
        *item = i;
    }
    box_array
});

pub async fn get_ncm_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut dir = tokio::fs::read_dir(path).await?;
    let mut ncm_files = Vec::new();
    while let Some(entry) = dir.next_entry().await? {
        let path = entry.path();
        if path.extension().unwrap() == "ncm" {
            ncm_files.push(path);
        }
    }
    Ok(ncm_files)
}

#[instrument(err)]
pub async fn decode_ncm(path: &Path, output_dir: &str) -> Result<()> {
    let path = Path::new(path);
    let name = path.file_stem().unwrap().to_string_lossy();
    let mut f = File::open(path).await?;
    let key_box = get_key_box(&mut f).await?;
    let format = get_music_format(&mut f).await?;
    // 跳过封面数据
    f.seek(SeekFrom::Current(9)).await?;
    read_data(&mut f).await?;
    fs::create_dir_all(output_dir).await?;
    let out_f = File::create(Path::new(output_dir).join(format!("{name}.{format}"))).await?;
    let mut bw = BufWriter::new(out_f);
    let mut br = BufReader::new(f);
    let mut buffer = [0; BUFFER_SIZE];
    loop {
        let len = br.read(&mut buffer).await?;
        if len == 0 {
            break;
        }
        buffer[..len].iter_mut().enumerate().for_each(|(i, byte)| {
            let j = (i + 1) as u8;
            let key_index =
                (key_box[j as usize] + key_box[(key_box[j as usize] + j as usize) & 0xff]) & 0xff;
            *byte ^= key_box[key_index] as u8;
        });

        bw.write_all(&buffer[..len]).await?;
        bw.flush().await?;
    }
    Ok(())
}
#[instrument(err)]
async fn get_key_box(f: &mut File) -> Result<KeyBox> {
    verify_ncm_header(f).await?;
    let mut key_data = read_data(f).await?;
    key_data.iter_mut().for_each(|item| *item ^= KEY_XOR_VALUE);
    let data: &[u8] = &CORE_CIPHER
        .clone()
        .decrypt_padded_mut::<Pkcs7>(&mut key_data)
        .unwrap()[17..];
    let key_length = data.len();
    let mut key_box = *KEY_BOX;
    let (mut c, mut last_byte, mut offset, mut swap) = (0, 0, 0, 0);
    (0..=255).for_each(|i| {
        swap = key_box[i];
        c = (swap + last_byte + data[offset] as usize) & 0xff;
        offset += 1;
        if offset >= key_length {
            offset = 0;
        }
        key_box[i] = key_box[c];
        key_box[c] = swap;
        last_byte = c;
    });
    Ok(key_box)
}
#[instrument(err, skip(f))]
async fn verify_ncm_header(f: &mut File) -> Result<()> {
    let mut header = [0; 8];
    f.read_exact(&mut header).await?;
    if header != NCM_HEADER {
        bail!("Invalid NCM file")
    }
    // 如果文件头通过校验, 跳过2字节的crc校验(一般用于加密)
    f.seek(SeekFrom::Current(2)).await?;
    Ok(())
}
#[instrument(err, skip(f))]
async fn read_data(f: &mut File) -> Result<Vec<u8>> {
    let mut length = [0; 4];
    f.read_exact(&mut length).await?;
    let mut data = vec![0; u32::from_le_bytes(length) as usize];
    f.read_exact(&mut data).await?;
    Ok(data)
}
#[instrument(err, skip(f))]
async fn get_music_format(f: &mut File) -> Result<String> {
    let mut meta_data = read_data(f).await?;
    meta_data
        .iter_mut()
        .for_each(|byte| *byte ^= META_XOR_VALUE);
    let mut meta_data = base64ct::Base64::decode_vec(&String::from_utf8(meta_data)?[22..])?;
    let meta_data = META_CIPHER
        .clone()
        .decrypt_padded_mut::<Pkcs7>(&mut meta_data)
        .map_err(|e| error!("{:?}", e))
        .unwrap();
    let meta_data = &String::from_utf8_lossy(meta_data)[6..];
    debug!("{}", meta_data);
    let meta_data: Value = serde_json::from_str(meta_data)?;
    Ok(meta_data["format"].as_str().unwrap().to_string())
}
