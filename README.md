# APPDATACleaner - 适用于 Windows 系统的 appdata 文件夹清理工具
完全开源免费的清理 Appdata 的小工具！完全使用 ChatGPT-4o 生成！

## 开发原因
  <details>
  <summary><h2>点击展开</h2></summary>
  <p>Windows系统安装的软件卸载时，即使使用了专业软件呢卸载工具卸载后，appdata 中的文件仍旧不会删除，故开发此软件清理。</p>
  <p>本工具使用 Rust 编写，使用 ChatGPT-4o 生成，并使用 egui 构建 GUI。</p>
  <p>本工具完全开源免费，欢迎各位大佬贡献代码。</p>
  </details>

## 🖥系统要求
- Windows

## 使用方法

### 📦下载exe文件
- [发行版](https://github.com/TC999/AppDataCleaner/releases/latest)
- [CI 构建](https://github.com/TC999/AppDataCleaner/actions/workflows/ci.yml)

以上两种方法二选一，下载后直接解压运行即可。

### 运行
> [!caution]
> 
> 请注意，删除操作不可逆，请谨慎操作。
- 双击运行
- 点击“立即扫描”,软件会自动扫描 Appdata 文件夹，并显示扫描结果。
- 自行选择“删除”或“移动”（暂未实现）

### 从源码编译
- 安装 Rust
- 克隆此仓库
```
git clone https://github.com/TC999/AppDataCleaner.git
```
- 进入项目目录
```
cd AppDataCleaner
```
- 调试
```
cargo run
```
- 编译
```
cargo build --release
```
- 编译产物在 target/release 目录下

## 代码结构说明
- `src`: 代码目录
- `assets`: 资源文件目录(注：字体文件不可删除，否则运行会显示方块！)

## ✔ 待办
- [ ] 移动文件夹
- [ ] 多国语言支持
- [ ] 优化界面
- [ ] 优化代码
- [ ] 移动文件
## ✨ 贡献

## 鸣谢
- [TC999](https://github.com/TC999) - 作者
- [ChatGPT-4o](https://chatgpt.com/) - 代码编写
- [egui](https://github.com/emilk/egui) - GUI 框架
## 📝 许可证
本项目采用 [GPLv3 许可证](LICENSE)。