SET OPENSSL_LIB_DIR=c:/OpenSSL-Win64
SET OPENSSL_INCLUDE_DIR=c:/OpenSSL-Win64/include
rem SET OPENSSL_STATIC=yes
rem -- -C link_args="-Wl,--subsystem,windows"
cargo build --release
pause
