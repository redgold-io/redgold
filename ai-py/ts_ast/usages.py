from dataclasses import dataclass
from pathlib import Path
from typing import Optional, List, Dict, Set
from pydantic import BaseModel
import toml

from ts_ast.ts_util import ts_read, get_type_id_name, get_identifier_name


class ImportSource(BaseModel):
    crate_name: str
    source_type: str  # 'external', 'workspace', 'local'
    source_path: Optional[str] = None
    file_path: Optional[str] = None  # The actual file path on disk where this import is defined


class RustImport(BaseModel):
    full_path: str  # The full import path e.g. crate::core::block::Block
    alias: Optional[str] = None  # The alias if using 'as', e.g. 'use x as y'
    start_line: int
    end_line: int
    source: Optional[ImportSource] = None
    # file_path: Optional[str] = None  # The actual file path on disk where this import is defined


class StructUsage(BaseModel):
    struct_name: str  # The name of the struct being used
    usage_file: str  # The file where the usage was found
    usage_line: int  # The line number where the usage was found
    usage_column: int
    context: str  # The line of code containing the usage


class ImportResolver:
    def __init__(self, cargo_toml_path: str):
        # print(f"Initializing resolver with cargo path: {cargo_toml_path}")
        self.cargo_toml_path = Path(cargo_toml_path)
        self.workspace_root = self.cargo_toml_path.parent
        # print(f"Workspace root: {self.workspace_root}")
        self.cargo_data = toml.load(str(cargo_toml_path))
        self.workspace_crates = {}  # name -> path mapping
        self.external_deps = set()
        self._init_mappings()
        # print("Workspace crates found:", self.workspace_crates)
        # print("External deps found:", self.external_deps)

    def _init_mappings(self):
        # First handle workspace dependencies section which maps crate names to paths
        if 'workspace' in self.cargo_data and 'dependencies' in self.cargo_data['workspace']:
            workspace_deps = self.cargo_data['workspace']['dependencies']
            # print("Found workspace dependencies:", workspace_deps)
            for name, spec in workspace_deps.items():
                if isinstance(spec, dict):
                    if 'path' in spec:
                        # This is a local workspace dependency
                        path = self.workspace_root / spec['path']
                        self.workspace_crates[name] = str(path)
                        # print(f"Added workspace dependency {name} -> {path}")
                    else:
                        # External dependency
                        self.external_deps.add(name)
                else:
                    self.external_deps.add(name)

        # Then handle workspace members
        if 'workspace' in self.cargo_data and 'members' in self.cargo_data['workspace']:
            # print("Found workspace members:", self.cargo_data['workspace']['members'])
            for member in self.cargo_data['workspace']['members']:
                member_path = self.workspace_root / member
                # print(f"Processing member {member} at {member_path}")
                if member_path.exists():
                    member_toml_path = member_path / 'Cargo.toml'
                    try:
                        if member_toml_path.exists():
                            member_toml = toml.load(str(member_toml_path))
                            if 'package' in member_toml and 'name' in member_toml['package']:
                                name = member_toml['package']['name']
                                self.workspace_crates[name] = str(member_path)
                                # print(f"Added workspace crate {name} -> {member_path}")
                        else:
                            # If no Cargo.toml, infer name from directory
                            name = member_path.name.replace('-', '_')
                            self.workspace_crates[name] = str(member_path)
                            # print(f"Inferred workspace crate {name} -> {member_path}")
                    except Exception as e:
                        # print(f"Warning: Failed to load {member_toml_path}: {e}")
                        name = member_path.name.replace('-', '_')
                        self.workspace_crates[name] = str(member_path)

        # Finally handle root crate dependencies
        self._add_deps_from_section(self.cargo_data, 'dependencies')

    def _add_deps_from_section(self, data: dict, section: str):
        if section in data:
            deps = data[section]
            for name, spec in deps.items():
                if isinstance(spec, dict) and 'path' in spec:
                    # This is a local dependency
                    path = Path(spec['path'])
                    if not path.is_absolute():
                        path = self.workspace_root / path
                    self.workspace_crates[name] = str(path)
                    # print(f"Added local dependency {name} -> {path}")
                else:
                    # This is an external dependency
                    self.external_deps.add(name)

    def _find_module_file(self, import_path: str, base_path: str) -> Optional[str]:
        """Find the actual file path for a module import."""
        # print(f"\nLooking for module file: {import_path} in {base_path}")
        parts = import_path.split('::')
        if len(parts) <= 1:
            return None

        # Remove crate prefix if present
        if parts[0] == 'crate':
            parts = parts[1:]
        elif parts[0] in self.workspace_crates:
            base_path = str(Path(self.workspace_crates[parts[0]]) / 'src')
            parts = parts[1:]
        elif parts[0] in ['std', 'core']:
            return None

        # Start from the base path
        current_path = Path(base_path)
        # print(f"Starting search from: {current_path}")
        # print(f"Looking for parts: {parts}")
        
        # For each part except the last one (which might be a type/struct name)
        for i, part in enumerate(parts[:-1]):
            # print(f"Processing part: {part}")
            # Try as a directory first
            dir_path = current_path / part
            if dir_path.is_dir():
                # Check for mod.rs
                mod_rs = dir_path / "mod.rs"
                if mod_rs.exists():
                    # print(f"Found mod.rs at {mod_rs}")
                    current_path = dir_path
                    continue
            
            # Try as a file
            file_path = current_path / f"{part}.rs"
            if file_path.exists():
                # print(f"Found {part}.rs at {file_path}")
                return str(file_path)  # Return immediately when we find the module file
                
            # If neither exists, try lib.rs in the directory
            lib_rs = dir_path / "lib.rs"
            if lib_rs.exists():
                # print(f"Found lib.rs at {lib_rs}")
                current_path = dir_path
                continue
                
            # print(f"Could not find module for part: {part}")
            return None  # Return None if we can't find a part
                
        # If we've processed all parts except the last and haven't returned,
        # check if we're in a directory and need to return its mod.rs
        if current_path.is_dir():
            mod_rs = current_path / "mod.rs"
            if mod_rs.exists():
                return str(mod_rs)
                
        return None

    def resolve_import(self, import_path: str, current_file: str) -> ImportSource:
        # print(f"\nResolving import: {import_path} from {current_file}")
        first_part = import_path.split('::')[0]
        first_part = first_part.replace("_", "-")
        current_file = Path(current_file)
        file_path = None
        
        # Handle crate-relative imports
        if first_part == 'crate':
            # print("Handling crate-relative import")
            # First check if we're in a workspace crate
            for crate_name, crate_path in self.workspace_crates.items():
                if str(current_file).startswith(str(Path(crate_path))):
                    file_path = self._find_module_file(import_path, str(Path(crate_path) / 'src'))
                    # print(f"Found in workspace crate {crate_name} -> {file_path}")
                    return ImportSource(
                        crate_name=crate_name,
                        source_type='local',
                        source_path=crate_path,
                        file_path=file_path
                    )
            
            # If not in a workspace crate, might be in the root crate
            if str(current_file).startswith(str(self.workspace_root)):
                root_name = self.cargo_data.get('package', {}).get('name', 'unknown')
                file_path = self._find_module_file(import_path, str(self.workspace_root / 'src'))
                # print(f"Found in root crate -> {file_path}")
                return ImportSource(
                    crate_name=root_name,
                    source_type='local',
                    source_path=str(self.workspace_root),
                    file_path=file_path
                )
            
            return ImportSource(
                crate_name='unknown',
                source_type='local',
                source_path=str(current_file.parent),
                file_path=None
            )


        # print(f"Checking workspace crates: {first_part} in {self.workspace_crates}")
        # Check workspace crates
        if first_part in self.workspace_crates:
            # print(f"Found workspace crate: {first_part}")
            crate_path = self.workspace_crates[first_part]
            # For workspace crates, we need to look in their src directory
            src_path = str(Path(crate_path) / 'src')
            # Remove the crate name from the path since we're already in its src directory
            remaining_path = '::'.join(import_path.split('::')[1:])
            if remaining_path:
                file_path = self._find_module_file(remaining_path, src_path)
            else:
                # If no remaining path, look for lib.rs or mod.rs
                for candidate in [Path(src_path) / 'lib.rs', Path(src_path) / 'mod.rs']:
                    if candidate.exists():
                        file_path = str(candidate)
                        break
            # print(f"Workspace crate file path: {file_path}")
            full = ImportSource(
                crate_name=first_part,
                source_type='workspace',
                source_path=crate_path,
                file_path=file_path
            )
            # print(f"Full: {full}")
            return full

        # Check external deps
        if first_part in self.external_deps:
            # print(f"Found external dependency: {first_part}")
            return ImportSource(
                crate_name=first_part,
                source_type='external',
                file_path=None
            )

        # Default case - might be a std lib import or unknown
        # print(f"Default case for: {first_part}")
        if first_part in ['std', 'core']:
            return ImportSource(
                crate_name=first_part,
                source_type='external',
                file_path=None
            )
            
        # If we get here and it's in the workspace dependencies, treat it as a workspace crate
        if first_part in self.workspace_crates:
            crate_path = self.workspace_crates[first_part]
            src_path = str(Path(crate_path) / 'src')
            remaining_path = '::'.join(import_path.split('::')[1:])
            file_path = self._find_module_file(remaining_path, src_path) if remaining_path else None
            return ImportSource(
                crate_name=first_part,
                source_type='workspace',
                source_path=crate_path,
                file_path=file_path
            )
            
        return ImportSource(
            crate_name=first_part,
            source_type='unknown',
            file_path=None
        )


def find_use_statements(node, resolver: ImportResolver, current_file: str) -> List[RustImport]:
    """Extract all use statements from a file's AST."""
    imports = []
    
    def collect_full_path(node):
        """Recursively collect the full path from a scoped identifier."""
        parts = []
        
        if node.type == 'identifier':
            return [node.text.decode('utf8')]
            
        if node.type == 'scoped_identifier':
            # Get the path part
            path_node = node.child_by_field_name('path')
            if path_node:
                parts.extend(collect_full_path(path_node))
            
            # Get the name part
            name_node = node.child_by_field_name('name')
            if name_node:
                parts.extend(collect_full_path(name_node))
                
        return parts

    def process_use_tree(use_node, start_line, end_line, prefix=""):
        # Handle basic use statements
        if use_node.type == 'identifier':
            path = use_node.text.decode('utf8')
            if path:  # Only add if path is not empty
                full_path = f"{prefix}{path}" if prefix else path
                source = resolver.resolve_import(full_path, current_file)
                return [RustImport(
                    full_path=full_path,
                    start_line=start_line,
                    end_line=end_line,
                    source=source
                )]
            return []
        
        # Handle scoped imports
        if use_node.type == 'scoped_identifier':
            path_parts = collect_full_path(use_node)
            if path_parts:  # Only create import if we found path parts
                full_path = f"{prefix}{'::'.join(path_parts)}" if prefix else '::'.join(path_parts)
                source = resolver.resolve_import(full_path, current_file)
                return [RustImport(
                    full_path=full_path,
                    start_line=start_line,
                    end_line=end_line,
                    source=source
                )]
            return []

        # Handle use trees (e.g., use std::{fs, io})
        if use_node.type == 'use_tree_list':
            results = []
            for child in use_node.children:
                if child.type not in [',', '{', '}']:  # Skip punctuation
                    child_imports = process_use_tree(child, start_line, end_line, prefix)
                    results.extend(child_imports)
            return results

        # Handle nested paths
        if use_node.type == 'use_tree':
            path = use_node.child_by_field_name('path')
            list_node = use_node.child_by_field_name('list')
            name = use_node.child_by_field_name('name')
            
            new_prefix = ""
            if path:
                path_parts = collect_full_path(path)
                if path_parts:
                    path_str = '::'.join(path_parts)
                    new_prefix = f"{prefix}{path_str}::" if prefix else f"{path_str}::"

            if list_node:
                return process_use_tree(list_node, start_line, end_line, new_prefix)
            elif name:
                return process_use_tree(name, start_line, end_line, new_prefix)
            elif path:  # Also handle single path without list or name
                return process_use_tree(path, start_line, end_line, prefix)
            
        return []

    for child in node.children:
        if child.type == 'use_declaration':
            tree = child.child_by_field_name('argument')
            if tree:
                imports.extend(process_use_tree(
                    tree,
                    child.start_point[0] + 1,
                    child.end_point[0] + 1
                ))
    
    return imports


def find_struct_usages(file_path: str, target_struct_name: str, imports: List[RustImport]) -> List[StructUsage]:
    """Find all usages of a struct in a given file."""
    usages = []
    root_node = ts_read(file_path)
    
    # Read the file content for context
    with open(file_path, 'r') as f:
        file_content = f.readlines()
    
    def process_node(node):
        if node.type == 'type_identifier':
            name = node.text.decode('utf8')
            if name == target_struct_name:
                line_num = node.start_point[0] + 1
                context = file_content[node.start_point[0]].strip()
                usages.append(StructUsage(
                    struct_name=target_struct_name,
                    usage_file=file_path,
                    usage_line=line_num,
                    usage_column=node.start_point[1] + 1,
                    context=context
                ))
        
        for child in node.children:
            process_node(child)
    
    process_node(root_node)
    return usages


def find_rust_imports(rust_files: List[str], cargo_toml_path: str) -> Dict[str, List[RustImport]]:
    """Find all imports in a list of Rust files."""
    resolver = ImportResolver(cargo_toml_path)
    imports: Dict[str, List[RustImport]] = {}
    for file_path in rust_files:
        root_node = ts_read(file_path)
        imports[file_path] = find_use_statements(root_node, resolver, file_path)
    return imports


def find_all_struct_usages(
        rust_files: List[str], 
        target_struct: str, 
        source_file: str,
        imports: Optional[Dict[str, List[RustImport]]] = None
        ) -> List[StructUsage]:
    """Find all usages of a struct across all Rust files."""
    all_usages = []
    
    file_imports = find_rust_imports(rust_files, 'Cargo.toml') if imports is None else imports

    # Then find usages in each file
    for file_path in rust_files:
        if file_path == source_file:
            continue  # Skip the source file itself
        
        file_usages = find_struct_usages(
            file_path,
            target_struct,
            file_imports[file_path]
        )
        all_usages.extend(file_usages)
    
    return all_usages
