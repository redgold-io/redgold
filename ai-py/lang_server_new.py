from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, cast, Any, Tuple, Union
import json

from multilspy import SyncLanguageServer
from multilspy.multilspy_config import MultilspyConfig
from multilspy.multilspy_logger import MultilspyLogger
from multilspy.multilspy_types import UnifiedSymbolInformation, Location, Range, Position

from repo_reader import WORKSPACE_PATH

@dataclass
class FileIndex:
    """Represents a complete index of a file's symbols and their locations"""
    symbols: List[UnifiedSymbolInformation]
    definitions: Dict[str, Location]  # symbol name -> location
    references: Dict[str, List[Location]]  # symbol name -> list of reference locations
    hover_info: Dict[str, str]  # symbol name -> hover documentation

class LangServerManager:
    def __init__(self, workspace_path: Path = WORKSPACE_PATH):
        self.workspace_path = workspace_path
        self.config = MultilspyConfig.from_dict({"code_language": "rust"})
        self.logger = MultilspyLogger()
        self._lsp: Optional[SyncLanguageServer] = None
        self._server_context = None
        self.file_indices: Dict[str, FileIndex] = {}
        
    def start(self):
        """Start the language server if not already running"""
        if self._lsp is None:
            self._lsp = SyncLanguageServer.create(self.config, self.logger, str(self.workspace_path))
            self._server_context = self._lsp.start_server()
            self._server_context.__enter__()
        return self
    
    def stop(self):
        """Stop the language server if running"""
        if self._server_context is not None:
            self._server_context.__exit__(None, None, None)
            self._server_context = None
            self._lsp = None
    
    def __enter__(self):
        return self.start()
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.stop()
    
    def index_file(self, relative_path: str) -> FileIndex:
        """Create a complete index of a file including all symbols, definitions, references and hover info"""
        if not self._lsp:
            raise RuntimeError("Language server not started")
            
        # Get all symbols in the file
        raw_response = self._lsp.request_document_symbols(relative_path)
        print("Raw symbols response:")
        print(json.dumps(raw_response, indent=2))
        
        # Handle tuple response (symbols, None)
        raw_symbols = []
        if isinstance(raw_response, tuple):
            raw_symbols = raw_response[0] if raw_response[0] is not None else []
        elif isinstance(raw_response, list):
            raw_symbols = raw_response
        
        # Initialize containers for the index
        symbols = []
        definitions = {}
        references = {}
        hover_info = {}
        
        # For each symbol, gather its definition, references and hover info
        try:
            for raw_symbol in raw_symbols:
                if isinstance(raw_symbol, dict) and "name" in raw_symbol:
                    symbol = cast(UnifiedSymbolInformation, raw_symbol)
                    symbols.append(symbol)
                    name = symbol["name"]
                    
                    # Get definition
                    try:
                        if "location" in symbol:
                            definitions[name] = symbol["location"]
                        else:
                            # Try to get definition through LSP
                            range_start = symbol.get("range", {}).get("start")
                            if range_start:
                                def_result = self._lsp.request_definition(
                                    relative_path, 
                                    range_start["line"], 
                                    range_start["character"]
                                )
                                if def_result:
                                    definitions[name] = def_result[0]
                    except Exception as e:
                        print(f"Failed to get definition for {name}: {e}")
                    
                    # Get references
                    try:
                        range_start = symbol.get("range", {}).get("start")
                        if range_start:
                            refs = self._lsp.request_references(
                                relative_path, 
                                range_start["line"], 
                                range_start["character"]
                            )
                            if refs:
                                references[name] = refs
                    except Exception as e:
                        print(f"Failed to get references for {name}: {e}")
                    
                    # Get hover info
                    try:
                        range_start = symbol.get("range", {}).get("start")
                        if range_start:
                            hover = self._lsp.request_hover(
                                relative_path, 
                                range_start["line"], 
                                range_start["character"]
                            )
                            if hover and "contents" in hover:
                                if isinstance(hover["contents"], str):
                                    hover_info[name] = hover["contents"]
                                elif isinstance(hover["contents"], dict):
                                    hover_info[name] = hover["contents"]["value"]
                                elif isinstance(hover["contents"], list):
                                    hover_info[name] = "\n".join(str(c) for c in hover["contents"])
                    except Exception as e:
                        print(f"Failed to get hover info for {name}: {e}")
        except Exception as e:
            print(f"Error processing symbols: {e}")
        
        # Create and store the index
        index = FileIndex(
            symbols=symbols,
            definitions=definitions,
            references=references,
            hover_info=hover_info
        )
        self.file_indices[relative_path] = index
        return index

    def get_completions(self, relative_path: str, line: int, character: int):
        """Get completions at a specific position"""
        if not self._lsp:
            raise RuntimeError("Language server not started")
        return self._lsp.request_completions(relative_path, line, character)
    
    def get_definition(self, relative_path: str, line: int, character: int):
        """Get definition of symbol at position"""
        if not self._lsp:
            raise RuntimeError("Language server not started")
        return self._lsp.request_definition(relative_path, line, character)
    
    def get_references(self, relative_path: str, line: int, character: int):
        """Get all references to symbol at position"""
        if not self._lsp:
            raise RuntimeError("Language server not started")
        return self._lsp.request_references(relative_path, line, character)
    
    def get_hover(self, relative_path: str, line: int, character: int):
        """Get hover information at position"""
        if not self._lsp:
            raise RuntimeError("Language server not started")
        return self._lsp.request_hover(relative_path, line, character)

# Example usage with test file
def test_lang_server():
    test_path = "src/core/relay.rs"
    
    with LangServerManager() as lsp:
        try:
            # Create complete index of the file
            index = lsp.index_file(test_path)
            print(f"\nFound {len(index.symbols)} symbols in {test_path}")
            for symbol in index.symbols:
                print(f"Symbol: {symbol['name']} ({symbol.get('kind')}) - {symbol.get('detail', '')}")
                if symbol['name'] in index.hover_info:
                    print(f"  Documentation: {index.hover_info[symbol['name']]}")
            print(f"Definitions: {len(index.definitions)}")
            print(f"References: {len(index.references)}")
            print(f"Hover info: {len(index.hover_info)}")
            
            # Example of using the persistent server for real-time operations
            completions = lsp.get_completions(test_path, 10, 0)
            if isinstance(completions, tuple):
                completions = completions[0] if completions[0] is not None else []
            print(f"Got {len(completions)} completions at line 10")
        except Exception as e:
            print(f"Error during indexing: {e}")
            import traceback
            traceback.print_exc()

if __name__ == "__main__":
    test_lang_server()
