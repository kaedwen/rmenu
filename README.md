# rmenu
Wayland base dymanic launcher fully written in rust. The launcher can de customized and collects a usage statistic to provide the best matching binary to launch.

# Building
```
cargo build --release
```

# Installing
```
cargo install --path .
```

# Customize
The launcher can be customized placing a config to `~/.config/rmenu/config`.

Have a look to the example config [here](example/config.yaml)

# License
* [GNU LGPLv3 (or any later version)](LICENSE)