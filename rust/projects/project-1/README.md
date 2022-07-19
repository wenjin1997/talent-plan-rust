## 安装`clap`步骤


* Step1: 安装 homebrew，参考[Homebrew](https://brew.sh/index_zh-cn)。

    我的是macos系统，为了有 xcode command line tool，在AppStroe下载了Xcode应用。
    ```bash
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.
    ```
    但是运行上述命令时老出现网络问题，可以使用下面的命令来安装：
    ```bash
    /bin/zsh -c "$(curl -fsSL https://gitee.com/cunkai/HomebrewCN/raw/master/Homebrew.sh)"
    ```
    参考[mac安装homebrew失败怎么办？](https://www.zhihu.com/question/35928898)。

* Step 2: 安装openssl，参考[如何在 Mac 操作系统上安装 libssl-dev、libffi-dev？](https://qa.1r1g.cn/superuser/ask/76257331/)
    ```bash
    brew install openssl
    ```
    但是安装时出现错误：
    ```
    ==> Installing dependencies for openssl@1.1: ca-certificates
    ==> Installing openssl@1.1 dependency: ca-certificates
    fatal: not in a git directory
    Error: Command failed with exit 128: git
    ```
    参考[MacOS下homebrew install报错：fatal: not in a git directory Error: Command failed with exit 128: git](https://blog.csdn.net/zhangzq86/article/details/125021669)，用`brew -v`来查看解决方案。

    安装后为：
    ```bash
    jinjin@Mac-mini ~ % brew -v
    Homebrew 3.5.4-74-g8dc46a7
    fatal: unsafe repository ('/opt/homebrew/Library/Taps/homebrew/homebrew-core' is owned by someone else)
    To add an exception for this directory, call:
    
        git config --global --add safe.directory /opt/homebrew/Library/Taps/homebrew/homebrew-core
    Homebrew/homebrew-core (no Git repository)
    Homebrew/homebrew-cask (git revision 8dc46a7c4; last commit 2022-07-14)
    jinjin@Mac-mini ~ % git config --global --add safe.directory /opt/homebrew/Library/Taps/homebrew/homebrew-core
    jinjin@Mac-mini ~ % brew install openssl@1.1
    Warning: No remote 'origin' in /opt/homebrew/Library/Taps/homebrew/homebrew-services, skipping update!
    ==> Downloading https://mirrors.aliyun.com/homebrew/homebrew-bottl
    Already downloaded: /Users/jinjin/Library/Caches/Homebrew/downloads/1437ed0ad3fe7c4a18ae479fad6608ac24096f1062b3d603b2281e0e6075edff--ca-certificates-2022-04-26.all.bottle.tar.gz
    ==> Downloading https://mirrors.aliyun.com/homebrew/homebrew-bottl
    Already downloaded: /Users/jinjin/Library/Caches/Homebrew/downloads/b098a1c9bb158c9ec5d756650b1c9b18b052f6bd06863d7f625c906693de5e64--openssl@1.1-1.1.1q.arm64_monterey.bottle.tar.gz
    ==> Installing dependencies for openssl@1.1: ca-certificates
    ==> Installing openssl@1.1 dependency: ca-certificates
    ==> Pouring ca-certificates-2022-04-26.all.bottle.tar.gz
    ==> Regenerating CA certificate bundle from keychain, this may tak
    🍺  /opt/homebrew/Cellar/ca-certificates/2022-04-26: 3 files, 215.6KB
    ==> Installing openssl@1.1
    ==> Pouring openssl@1.1-1.1.1q.arm64_monterey.bottle.tar.gz
    ==> Caveats
    A CA file has been bootstrapped using certificates from the system
    keychain. To add additional certificates, place .pem files in
    /opt/homebrew/etc/openssl@1.1/certs
    
    and run
    /opt/homebrew/opt/openssl@1.1/bin/c_rehash
    
    openssl@1.1 is keg-only, which means it was not symlinked into /opt/homebrew,
    because macOS provides LibreSSL.
    
    If you need to have openssl@1.1 first in your PATH, run:
    echo 'export PATH="/opt/homebrew/opt/openssl@1.1/bin:$PATH"' >> ~/.zshrc
    
    For compilers to find openssl@1.1 you may need to set:
    export LDFLAGS="-L/opt/homebrew/opt/openssl@1.1/lib"
    export CPPFLAGS="-I/opt/homebrew/opt/openssl@1.1/include"
    
    ==> Summary
    🍺  /opt/homebrew/Cellar/openssl@1.1/1.1.1q: 8,097 files, 18MB
    ==> Running `brew cleanup openssl@1.1`...
    Disable this behaviour by setting HOMEBREW_NO_INSTALL_CLEANUP.
    Hide these hints with HOMEBREW_NO_ENV_HINTS (see `man brew`).
    ==> Caveats
    ==> openssl@1.1
    A CA file has been bootstrapped using certificates from the system
    keychain. To add additional certificates, place .pem files in
    /opt/homebrew/etc/openssl@1.1/certs
    
    and run
    /opt/homebrew/opt/openssl@1.1/bin/c_rehash
    
    openssl@1.1 is keg-only, which means it was not symlinked into /opt/homebrew,
    because macOS provides LibreSSL.
    
    If you need to have openssl@1.1 first in your PATH, run:
    echo 'export PATH="/opt/homebrew/opt/openssl@1.1/bin:$PATH"' >> ~/.zshrc
    
    For compilers to find openssl@1.1 you may need to set:
    export LDFLAGS="-L/opt/homebrew/opt/openssl@1.1/lib"
    export CPPFLAGS="-I/opt/homebrew/opt/openssl@1.1/include"
    ```

    参考[mac 安装pkg-config](https://blog.csdn.net/dyx810601/article/details/79911068) 。

* Step 3: 终于可以开始安装 `cargo-edit` 了。
    ```bash
    cargo install cargo-edit
    ```

* 使用 clap 的 derive 属性，执行：
    ```rust
    cargo add clap -F derive
    ```

## 命令行编程

关于命令行编程的资料可以参考[Command line apps in Rust](https://rust-cli.github.io/book/index.html#command-line-apps-in-rust)。一些代码见[grrs](/rust/building-blocks/bb-1/grrs/src/main.rs)。

命令行参数运行代码，如果要运行在`src/bin/kvs.rs`中的代码，可以这样运行：

```rust
cargo run --bin kvs -- --help
```

在[clap](https://github.com/clap-rs/clap)包中新增加了属性，会比之前的版本调用更加方便。在[clap-demo](/rust/building-blocks/bb-1/clap-demo/src/bin/clap_demo.rs)中有一些例子。

## `Clippy`和`rustfmt`

`cargo clippy` 可以给出一些有帮助的建议。

`cargo rustfmt` 可以让程序代码格式更好。

## Notes

`#![deny(missing_docs)]` 加在 `lib.rs` 文件的最上面，可以要求所有公开的模块或者函数都要有文档注释。
