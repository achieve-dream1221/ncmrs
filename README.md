# ncm 文件解码

这是一个使用 Rust 编写的异步 ncm 文件解码器, 参考自 [ncmpp](https://github.com/Majjcom/ncmpp)

## Usage

```
ncm 解码器

Usage: ncmrs.exe [OPTIONS] --input <INPUT> --output <OUTPUT>

Options:
  -i, --input <INPUT>    输入ncm文件/ncm目录
  -o, --output <OUTPUT>  输出目录
  -v, --verbose...       Increase logging verbosity
  -q, --quiet...         Decrease logging verbosity
  -h, --help             Print help
```

## Example

```shell
ncmrs.exe -i "path/to/ncm/file.ncm" -o "path/to/output"
```

## Thanks

- [ncmpp](https://github.com/Majjcom/ncmpp)