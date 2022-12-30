#!/usr/bin/env bash

# golang 版本
go version

if [[ "$OSTYPE" =~ ^darwin ]];then
    LIB="libsqlex.dylib"
    if [[ $(uname -m) == 'arm64' ]]; then
      DIST="../../native/darwin-aarch64/src/main/resources/native/darwin/aarch64"
    else
      DIST="../../native/darwin-amd64/src/main/resources/native/darwin/amd64"
    fi
elif [[ "$OSTYPE" =~ ^linux ]]; then
    LIB="libsqlex.so"
    DIST="../../native/linux-amd64/src/main/resources/native/linux/amd64"
else
  echo "不支持的操作系统,目前仅支持macos/linux"
  exit 1
fi

echo "构建原生库"
#构建原生库
cd 3rdparty/mysql && go build -buildmode c-shared -o $DIST/$LIB github.com/pingcap/tidb/sqlex

