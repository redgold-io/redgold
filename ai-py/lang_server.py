from pathlib import Path

from monitors4codegen.multilspy import SyncLanguageServer
from monitors4codegen.multilspy.multilspy_config import MultilspyConfig
from monitors4codegen.multilspy.multilspy_logger import MultilspyLogger
config = MultilspyConfig.from_dict({"code_language": "rust"})  # Also supports "python", "rust", "csharp"
logger = MultilspyLogger()
parent = Path.cwd().parent
print("repo path", parent)
lsp = SyncLanguageServer.create(config, logger, str(parent))
with lsp.start_server():
    result = lsp.request_definition(
        "src/main.rs", # Filename of location where request is being made
        4, # line number of symbol for which request is being made
        12 # column number of symbol for which request is being made
    )
    print(result)
    # result2 = lsp.request_completions(
    #     ...
    # )
    # result3 = lsp.request_references(
    #     ...
    # )
    # result4 = lsp.request_document_symbols(
    #     ...
    # )
    # result5 = lsp.request_hover(
    #     ...
    # )
    # ...