from collections import defaultdict
from copy import deepcopy
import hashlib
from pathlib import Path
from typing import Dict, List, Optional
from langchain_anthropic import ChatAnthropic
from pydantic import BaseModel, Field
from agent_loop import CLAUDE_SONNET
from repo_reader import AccumFileData, count_tokens
from langchain_community.embeddings import OpenAIEmbeddings
from langchain.text_splitter import RecursiveCharacterTextSplitter

from ts_ast.ts_functions import RustFunction, extract_all_functions
from ts_ast.ts_structs import RustStruct, extract_all_structs
from ts_ast.usages import RustImport, find_all_struct_usages, find_rust_imports, find_use_statements


a = AccumFileData.default()
WORKSPACE = Path.home() / "ai" / "redgold"



class EnrichmentIndex(BaseModel):
    file_imports: Dict[str, List[RustImport]] = {}
    file_structs: Dict[str, List[RustStruct]] = {}
    file_functions: Dict[str, List[RustFunction]] = {}
    good_files: List[str] = []



class EnrichmentConfig(BaseModel):
    """Configuration for code chunking strategy"""
    chunk_size: int = Field(
        default=2000,  # About 500 words / ~20-30 lines of code
        description="Target size of each chunk in characters"
    )
    chunk_overlap: int = Field(
        default=400,  # 20% overlap for context continuity
        description="Number of characters to overlap between chunks"
    )
    separators: List[str] = Field(
        default=["\nclass ", "\ndef ", "\n\n", "\n", " ", ""],
        description="Separators to use for chunking, in order of preference"
    )

    calculate_embeddings: bool = Field(
        default=True,
        description="Whether to calculate embeddings for the chunks"
    )

    workspace: str = Field(
        default=str(WORKSPACE),
        description="Workspace to use for enrichment"
    )
    


class CodeChunk(BaseModel):
    text: str
    start_line: int
    end_line: int
    embedding: Optional[List[float]] = None


class EnrichedFile(BaseModel):
    path: str = Field(default="")
    workspace: str = Field(default=str(WORKSPACE))
    relative_path: str = Field(default="")
    contents: str = Field(default="")
    lines: list[str] = Field(default_factory=list)
    hash: str = Field(default="")
    token_count: int = Field(default=0)
    chunks: List[CodeChunk] = Field(default_factory=list)
    naive_llm_single_file_summary: str = Field(default=""),
    functions: List[RustFunction] = Field(default_factory=list),
    function_names: List[str] = Field(default_factory=list)
    size_bytes: int = Field(default=0)
    structs: List[RustStruct] = Field(default_factory=list)
    struct_names: List[str] = Field(default_factory=list),
    imports: List[RustImport] = Field(default_factory=list)
    used_as_import_in_files: List[str] = Field(default_factory=list)

    def debug_str(self):
        return f"""{self.relative_path} {self.token_count} tokens {len(self.imports)} imports 
        {len(self.structs)} structs {len(self.functions)} functions {len(self.used_as_import_in_files)} 
        used as import in {len(self.used_as_import_in_files)} files"""
    
    def minus_chunks(self):
        c = deepcopy(self)
        c.chunks = []
        return c

    @classmethod
    def from_path(
        cls, 
        path_t: Path, 
        cfg: Optional[EnrichmentConfig] = None,
        previous_file: Optional["EnrichedFile"] = None,
        index: Optional[EnrichmentIndex] = None
        ) -> Optional["EnrichedFile"]:
        
        # Use default config if none provided
        if cfg is None:
            cfg = EnrichmentConfig()

        path = str(path_t)

        contents = path_t.read_text()
        hash = hashlib.sha256(contents.encode()).hexdigest()

        hash_same = False
        ef = EnrichedFile()
        # First check if the file hash is the same as the previous file
        if previous_file and previous_file.hash == hash:
            hash_same = True
            ef = previous_file

        if not hash_same:
            ef.path = path
            ef.hash = hash
            ef.workspace = cfg.workspace
            ef.relative_path = path_t.relative_to(cfg.workspace)
            ef.contents = contents
            ef.size_bytes = path_t.stat().st_size
            ef.token_count = count_tokens(contents)
            ef.lines = contents.split("\n")
            
            # Initialize text splitter
            text_splitter = RecursiveCharacterTextSplitter(
                chunk_size=cfg.chunk_size,
                chunk_overlap=cfg.chunk_overlap,
                length_function=len,
                separators=cfg.separators
            )
            
            # Split text into chunks
            chunks = text_splitter.create_documents([contents])
            
            # Create CodeChunk objects
            code_chunks = []
            embeddings_model = OpenAIEmbeddings()
            
            for chunk in chunks:
                # Find start and end lines for this chunk
                chunk_text = chunk.page_content
                start_idx = contents.find(chunk_text)
                end_idx = start_idx + len(chunk_text)
                
                start_line = contents[:start_idx].count('\n') + 1
                end_line = contents[:end_idx].count('\n') + 1
                
                # Get embedding for chunk
                embedding = embeddings_model.embed_query(chunk_text)
                
                code_chunks.append(CodeChunk(
                    text=chunk_text,
                    start_line=start_line,
                    end_line=end_line,
                    embedding=embedding
                ))
            ef.chunks = code_chunks

            ef.functions = extract_all_functions(str(path))
            ef.function_names = [f.name for f in ef.functions]

            ef.structs = extract_all_structs(str(path))
            ef.struct_names = [s.name for s in ef.structs]

            summary = ChatAnthropic(model=CLAUDE_SONNET).invoke(
                "Summarize the following code file: " + contents
            )
            ef.naive_llm_single_file_summary = str(summary.content)

            ef.imports = find_rust_imports([str(path)], str(Path(cfg.workspace) / "Cargo.toml"))[str(path)]
            
        return ef



class EnrichedRepository(BaseModel):
    cache_path: str = Field(default=str(Path.home() / "test_cache.json"))
    cfg: EnrichmentConfig = Field(default=EnrichmentConfig())
    files: List[EnrichedFile] = Field(default_factory=list)
    index: EnrichmentIndex = Field(default_factory=EnrichmentIndex)

    def get_prior_file(self, path: str) -> Optional[EnrichedFile]:
        for f in self.files:
            if f.path == path:
                return f
        return None
    

    def update_usages(self):
        usages = defaultdict(list)
        for f in self.files:
            for k in f.imports:
                if k.source:
                    usages[k.source.file_path].append(f.path)
        for f in self.files:
            f.used_as_import_in_files = usages[f.path]
                    
    
    def with_updated_file(self, path: Path, opt_iter_cache_update: int = 0) -> Optional[EnrichedFile]:
        new_var = self.get_prior_file(str(path))
        print(f"Updating file: {path} with prior file hash: {new_var.hash if new_var else 'None'}")
        ef = EnrichedFile.from_path(path, self.cfg, new_var, self.index)
        # print(ef.minus_chunks())
        self.files = [f for f in self.files if f.path != path]
        self.files.append(ef)
        if opt_iter_cache_update % 10 == 0 and new_var is None:
            self.update_cache()
        return ef
    
    def load_from_cache(self):
        json_data = Path(self.cache_path).read_text()
        try:
            # Try Pydantic V2 method first
            data = EnrichedRepository.model_validate_json(json_data)
        except AttributeError:
            # Fall back to Pydantic V1 method
            data = EnrichedRepository.parse_raw(json_data)
        return data
    @classmethod
    def new(cls, skip_cache: bool = False, calculate_embeddings: bool = True) -> "EnrichedRepository":

        inst = EnrichedRepository()
        if not skip_cache and Path(inst.cache_path).exists():
            print(f"Loading from cache: {inst.cache_path}")
            inst = inst.load_from_cache()
        
        a = AccumFileData.default()

        inst.cfg.calculate_embeddings = calculate_embeddings
        i = 0
        for f in a.good_files:
            i += 1
            inst = inst.with_updated_file(f, opt_iter_cache_update=i)
        inst.update_usages()
        # now update the index
        # index = EnrichmentIndex()
        # for f in updated_files:
        #     index.file_imports[f.path] = f.im
        #     index.file_structs[f.path] = f.structs
        # update cache:
        inst.update_cache()

        return inst
    def update_cache(self):
        try:
            # Try Pydantic V2 method first
            json_data = self.model_dump_json()
        except AttributeError:
            # Fall back to Pydantic V1 method
            json_data = self.json()
        Path(self.cache_path).write_text(json_data)




# vector_store = PGVector(
#     embeddings=embeddings,
#     collection_name="my_docs",
#     connection="postgresql+psycopg://...",
# )

test_path = WORKSPACE / "src" / "core" / "relay.rs"

def debug():
    test_path

    for r in extract_all_structs(str(test_path)):
        print(r.debug_str())

# debug()


good_files = a.good_files
rs_files = [str(f) for f in good_files if f.suffix == ".rs"]




def main():
    # Find all Cargo.toml files
    er = EnrichedRepository()
    # Load from JSON cache
    # json_data = Path(er.cache_path).read_text()
    # try:
    #     # Try Pydantic V2 method first
    #     data = EnrichedRepository.model_validate_json(json_data)
    # except AttributeError:
    #     # Fall back to Pydantic V1 method
    #     data = EnrichedRepository.parse_raw(json_data)
    # inst = data
    # print(inst)
    # er = EnrichedRepository.new()
    # er = EnrichedRepository.new()
    er = EnrichedRepository().load_from_cache()

    for f in er.files:
        f.chunks = []
        print(f.path)

    # import_files = [k for k in good_files if k.name == "Cargo.toml"]
    
    # root_import = [k for k in import_files if k.parent == working_dir][0]
    
    # print(f"Using root Cargo.toml: {root_import}")
    # print(f"Processing file: {test_path}")
    
    # # Get imports with source information
    # imports = find_rust_imports([str(test_path)], str(root_import))
    
    # # Print imports with their sources
    # for file_path, file_imports in imports.items():
    #     for imp in file_imports:
    #         print(f"Import: {imp.full_path}")
    #         print(f"Import: {imp}")
    #     print(f"\nImports for {file_path}:")
    #     for imp in file_imports:
    #         source_info = imp.source
    #         if source_info:
    #             print(f"  {imp.full_path}")
    #             print(f"    Type: {source_info.source_type}")
    #             print(f"    Crate: {source_info.crate_name}")
    #             if source_info.source_path:
    #                 print(f"    Source Path: {source_info.source_path}")
    #             if source_info.file_path:
    #                 print(f"    File: {source_info.file_path}")
    #             print(f"    Lines: {imp.start_line}-{imp.end_line}")
    #             print()


main()

# for path in [test_path]:
#     cfg = EnrichmentConfig.default()
#     cfg.chunk_overlap
#     enriched_file = EnrichedFile.from_path(path)    
#     print(enriched_file)