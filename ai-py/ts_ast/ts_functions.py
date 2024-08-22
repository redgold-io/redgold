import json
import os
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Optional, TypedDict

from repo_reader import AccumFileData
from ts_ast.ts_util import ts_read, child_functions, impl_items, get_identifier_name, get_impl_details, find_functions


def find_rust_function_exact():
    tooldef = {
        "name": "find_rust_function_exact",
        "description": """This tool indexes all rust files""",
        "input_schema": {
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": """The exact function name to look up, must be exactly equivalent to the function name"""
                },
                "path": {
                    "type": "string",
                    "description": """Optional path to restrict the search to"""
                },
                "impl": {
                    "type": "string",
                    "description": """Optional impl name to restrict the search to, for example, Relay::check_rate_limit; Relay would be the impl name"""
                },
                "trait": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": """Optional trait to restrict the function defs to, for example in `impl Default for Relay`, Relay would be the trait"""
                },
            },
            "required": ["name"]
        }
    }

    def find_function_exact_inner(input):

        # dependencies here, deal with clean reloading later
        ac = AccumFileData.default()
        good_files = [str(k) for k in ac.good_files if str(k).endswith('.rs')]

        name = input['name']
        path = input.get('path', None)
        impl = input.get("impl", None)
        trait = input.get("trait", None)

        if path is not None and path in good_files:
            good_files = [path]
        ret = []
        for g in good_files:
            for rf in extract_all_functions(g):
                if rf.name == name:
                    if impl is not None and rf.impl_name != impl:
                        continue
                    if trait is not None and rf.trait_name not in trait:
                        continue
                    ret.append(rf)
        return [r.llm_ser() for r in ret]
    return tooldef, find_function_exact_inner


@dataclass
class RustFunction:
    filename: str
    name: str
    start_line: int
    start_column: int
    end_line: int
    end_column: int
    content: str
    impl_name: Optional[str]
    trait_name: Optional[str]

    def debug_str(self):
        r = self
        return f"L{r.start_line}-{r.end_line} {r.scoped_fn_name()} {r.trait_name}"

    def scoped_fn_name(self):
        if self.impl_name is not None:
            return f"{self.impl_name}::{self.name}"
        return self.name

    def llm_ser(self):
        start = self.debug_str()
        return start + "\n" + self.content

def extract_all_functions(file_path) -> list[RustFunction]:
    ret = []
    root_node = ts_read(file_path)

    # Read the entire file content
    with open(file_path, 'r') as file:
        file_content = file.read()


    # types = defaultdict(int)
    # for child in root_node.children:
    #     types[child.type] += 1
    # print(types)

    # Find all function definitions
    function_nodes = child_functions(root_node)

    for func in function_nodes:
        func_name = get_identifier_name(func)
        ret.append(RustFunction(filename=file_path, name=func_name, start_line=func.start_point.row + 1,
                     start_column=func.start_point.column + 1, end_line=func.end_point.row + 1,
                     end_column=func.end_point.column + 1, content=file_content[func.start_byte:func.end_byte],
                                impl_name=None, trait_name=None))

    impl_nodes = impl_items(root_node)
    for node in impl_nodes:
        impl_name, trait_name = get_impl_details(node)
        # print("impl name", impl_name, trait_name)
        # print(get_identifier_name(node))
        # print ("-"*10)
        function_items = find_functions(node)
        for func in function_items:
            function_code = file_content[func.start_byte:func.end_byte]
            method_name = get_identifier_name(func)
            s = func.start_point
            e = func.end_point
            # print(method_name)
            rf = RustFunction(
                filename=file_path, name=method_name, start_line=s.row + 1, start_column=s.column + 1,
                end_line=e.row + 1, end_column=e.column + 1, content=function_code, impl_name=impl_name,
                trait_name=trait_name
            )
            ret.append(rf)

    return ret



def main():
    # Usage
    par_dir = Path.cwd().parent.parent
    repo_path = str(par_dir)
    # test_file = os.path.join(repo_path, 'schema/src/block.rs')
    test_file = os.path.join(repo_path, 'src/core/relay.rs')

    for r in extract_all_functions(test_file):
        print(r.debug_str())


if __name__ == '__main__':
    main()