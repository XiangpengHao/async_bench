# A view of async memory access in rust

This repo contains all the code to reproduce this post: https://blog.haoxp.xyz/posts/async-memory-access/.

## Build
```shell
cargo build --{release|debug}
```

## Run
```shell
./target/{release|debug}/async_bench --traveller {"async"| "sync"}
```

## Test
```shell
cargo test --{release|debug}
```


