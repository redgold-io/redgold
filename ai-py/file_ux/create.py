from pathlib import Path

def create_file(input):
    rel: str = input['repository_relative_path']
    content: str = input['content']
    include_as_rust_pub_mod: bool = input.get('include_as_rust_pub_mod', True)

    def inner():
        with open(rel, "w") as f:
            f.write(content)
        if include_as_rust_pub_mod:
            # first check if libs.rs exists in the same parent directory
            # if not, check for mod.rs
            rel_p = Path(rel)
            fnm = rel_p.name
            cur = rel_p.parent
            lib = cur.joinpath("lib.rs")
            mod = cur.joinpath("mod.rs")
            export_str = f"\npub mod {fnm.split('.')[0]};\n"
            if lib.exists():
                with open(lib, "a") as f:
                    f.write(export_str)
            elif mod.exists():
                with open(mod, "a") as f:
                    f.write(export_str)
            else:
                print("No lib.rs or mod.rs found in the parent directory, not adding the export line")
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