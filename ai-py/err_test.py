from google.protobuf.json_format import MessageToJson, Parse

import rg_proto.structs.structs_pb2 as structs


def main():
    err = structs.ErrorInfo(
     message="yo"
    )
    print(MessageToJson(err))
    print(Parse(MessageToJson(err), structs.ErrorInfo()))


if __name__ == "__main__":
    main()