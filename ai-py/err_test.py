import rg_proto.structs.structs_pb2 as structs

def main():
    err = structs.ErrorInfo()
    err.message = "This is an error message"
    print(err)

if __name__ == "__main__":
    main()