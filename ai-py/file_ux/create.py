

def edit_file_replace_lines_tooldef():
    return


def create_file(input):
    rel: str = input['repository_relative_path']
    content: str = input['content']
    include_as_rust_pub_mod: bool = input.get('include_as_rust_pub_mod', True)

    def inner():
        with open(rel, "w") as f:
            f.write(content)
        if include_as_rust_pub_mod:
            with open("lib.rs", "a") as f:
                f.write(f"pub mod {rel.split('.')[0]};\n")

    tool = {
        "name": "create_file",
        "description": """
        Create a file with the given content. This is primarily intended for creating .rs rust files, 
        as it includes an automatic helper function to include this new file in the compile path by 
        including it as a `pub mod` in the `lib.rs` or mod.rs module file.
        """,
        "input_schema": {
            "type": "object",
            "properties": {
                "repository_relative_path": {
                    "type": "string",
                    "description": """A relative path to be joined to the current active repository. For example, src/my_new_file.rs"""
                },
                "content": {
                    "type": "string",
                    "description": """The file contents to write to the new file"""
                },
                "include_as_rust_pub_mod": {
                    "type": "boolean",
                    "description": """Default true, automatically adds a pub mod line to lib.rs or mod.rs of cwd"""
                },
            },
            "required": ["repository_relative_path", "content"]
        }
    }
    return tool, inner