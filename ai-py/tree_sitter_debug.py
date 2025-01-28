import os
# Make sure you have the tree-sitter-rust library installed
import tree_sitter_rust
from pathlib import Path
from tree_sitter import Language, Parser

# Initialize the Rust language
RUST_LANGUAGE = Language(tree_sitter_rust.language())

parser = Parser()
parser.set_language(RUST_LANGUAGE)

def analyze_rust_file(file_path):
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
    except UnicodeDecodeError:
        print(f"Error: Unable to read {file_path} with UTF-8 encoding. Skipping.")
        return

    tree = parser.parse(bytes(content, 'utf-8'))
    root_node = tree.root_node
    print(f"\nFile: {file_path}")
    print(f"AST root type: {root_node.type}")
    print(f"Number of children: {len(root_node.children)}")

    # Find all function definitions
    function_nodes = [node for node in root_node.children if node.type == 'function_item']
    print(f"Number of functions: {len(function_nodes)}")

    for func in function_nodes:
        func_name = next((child.text.decode('utf8') for child in func.children if child.type == 'identifier'), "Unknown")
        print(f"Function name: {func_name}")

    # Find all struct definitions
    struct_nodes = [node for node in root_node.children if node.type == 'struct_item']
    print(f"Number of structs: {len(struct_nodes)}")

    for struct in struct_nodes:
        struct_name = next((child.text.decode('utf8') for child in struct.children if child.type == 'type_identifier'), "Unknown")
        print(f"Struct name: {struct_name}")


def analyze_rust_repo(repo_path):
    for root, dirs, files in os.walk(repo_path):
        for file in files:
            if file.endswith('.rs'):
                file_path = os.path.join(root, file)
                analyze_rust_file(file_path)

# Usage
par_dir = Path.cwd().parent
repo_path = str(par_dir)
test_file = os.path.join(repo_path, 'src/core/relay.rs')

analyze_rust_file(test_file)
# analyze_rust_repo(repo_path)