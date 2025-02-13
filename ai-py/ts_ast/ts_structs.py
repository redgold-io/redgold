from dataclasses import dataclass
from pathlib import Path
from typing import Optional, List

from pydantic import BaseModel
from ts_ast.ts_util import ts_read, struct_items, get_type_id_name, impl_items, get_impl_details


class RustField(BaseModel):
    name: str
    type: str
    start_line: int
    start_column: int
    end_line: int
    end_column: int
    content: str


class RustStruct(BaseModel):
    filename: str
    name: str
    start_line: int
    start_column: int
    end_line: int
    end_column: int
    content: str
    fields: List[RustField] = []

    def debug_str(self):
        r = self
        fields = [f"{f.name}: {f.type}" for f in self.fields]
        fields_str = ", ".join(fields) if fields else "no fields"
        return f"L{r.start_line}-{r.end_line} {r.name} [{fields_str}]"

    def llm_ser(self):
        start = self.debug_str()
        return start + "\n" + self.content


def get_field_info(field_node) -> Optional[RustField]:
    # Field declaration pattern: identifier: type
    field_name = None
    field_type = None
    
    for child in field_node.children:
        if child.type == 'field_identifier':
            field_name = child.text.decode('utf8')
        elif child.type == 'type_identifier' or child.type == 'primitive_type':
            field_type = child.text.decode('utf8')
        # Handle generic types like Vec<T>
        elif child.type == 'generic_type':
            field_type = child.text.decode('utf8')
        # Handle reference types like &str
        elif child.type == 'reference_type':
            field_type = child.text.decode('utf8')
    
    if field_name and field_type:
        return RustField(
            name=field_name,
            type=field_type,
            start_line=field_node.start_point.row + 1,
            start_column=field_node.start_point.column + 1,
            end_line=field_node.end_point.row + 1,
            end_column=field_node.end_point.column + 1,
            content=field_node.text.decode('utf8')
        )
    return None


def extract_all_structs(file_path) -> list[RustStruct]:
    ret = []
    root_node = ts_read(file_path)

    # Read the entire file content
    with open(file_path, 'r') as file:
        file_content = file.read()

    # Find all struct definitions
    struct_nodes = struct_items(root_node)

    for struct in struct_nodes:
        struct_name = get_type_id_name(struct)
        if struct_name:
            # Find the field declaration list node
            field_list = None
            for child in struct.children:
                if child.type == 'field_declaration_list':
                    field_list = child
                    break
            
            fields = []
            if field_list:
                for field_node in field_list.children:
                    if field_node.type == 'field_declaration':
                        field = get_field_info(field_node)
                        if field:
                            fields.append(field)
            
            rust_struct = RustStruct(
                filename=file_path,
                name=struct_name,
                start_line=struct.start_point.row + 1,
                start_column=struct.start_point.column + 1,
                end_line=struct.end_point.row + 1,
                end_column=struct.end_point.column + 1,
                content=file_content[struct.start_byte:struct.end_byte],
                fields=fields
            )
            ret.append(rust_struct)

    return ret


def main():
    # Usage example
    par_dir = Path.cwd().parent.parent
    test_file = str(par_dir / 'src' / 'core' / 'relay.rs')

    for r in extract_all_structs(test_file):
        print(r.debug_str())


if __name__ == '__main__':
    main() 