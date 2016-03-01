SET OPENSSL_LIB_DIR=c:/OpenSSL-Win64
SET OPENSSL_INCLUDE_DIR=c:/OpenSSL-Win64/include
rem SET OPENSSL_STATIC=yes
cargo build --release
pause
