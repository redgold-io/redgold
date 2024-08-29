# sudo apt install protobuf-compiler -y

~/protoc --experimental_allow_proto3_optional \
--proto_path=../schema/src/proto \
--python_out=rg_proto/structs \
--mypy_out=rg_proto/structs \
../schema/src/proto/structs.proto

