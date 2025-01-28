
# Make sure you have the tree-sitter-rust library installed
import tree_sitter_rust
from tree_sitter import Language, Parser

# Initialize the Rust language
RUST_LANGUAGE = Language(tree_sitter_rust.language())

parser = Parser()
parser.set_language(RUST_LANGUAGE)


def ts_read(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    tree = parser.parse(bytes(content, 'utf-8'))
    root_node = tree.root_node
    return root_node


def child_functions(root_node):
    return [node for node in root_node.children if node.type == 'function_item']


def struct_items(root_node):
    return [node for node in root_node.children if node.type == 'struct_item']


def impl_items(root_node):
    return [node for node in root_node.children if node.type == 'impl_item']


def get_type_id_name(struct):
    return next((child.text.decode('utf8') for child in struct.children if child.type == 'type_identifier'),
                None)


def get_generic_name(struct):
    return next((child.text.decode('utf8') for child in struct.children if child.type == 'generic_type'),
                None)


def get_second_generic_name(struct):
    type_ = list(child.text.decode('utf8') for child in struct.children if child.type == 'generic_type')
    if len(type_) > 1:
        return type_[1]
    return None


def get_type_id_or_generic_name(struct):
    ret = get_type_id_name(struct)
    if ret is None:
        ret = get_generic_name(struct)
    return ret


def get_identifier_name(func):
    return next((c.text.decode('utf8') for c in func.children if c.type == 'identifier'), None)


def get_impl_details(node):
    impl_name = get_type_id_or_generic_name(node)
    for_type = None

    for child in node.children:
        if child.type == 'trait':
            trait_child = child.child_by_field_name('name')
            if trait_child:
                impl_name = trait_child.text.decode('utf8')
        elif child.type == 'for':
            for_child = child.next_named_sibling
            if for_child and for_child.type == 'type_identifier':
                for_type = for_child.text.decode('utf8')
    if for_type is None:
        for_type = get_second_generic_name(node)

    return impl_name, for_type


def find_functions(node):
    functions = []
    if node.type == 'function_item':
        functions.append(node)
    for child in node.children:
        functions.extend(find_functions(child))
    return functions
