import dataclasses
import tiktoken
from pathlib import Path

from file_ux.fs_watcher import GitignoreFilter


def directory_exclusions():
    return [
        ".git", "target", "node_modules", ".idea", "experiments", "venv", ".nuxt", ".output", "dist",
        ".sqlx",
        "ignore-data",
        ".venv",
        "safe-bindings",
        "debug_scripts",
        "tests_backup"
    ]

def endings_exclusions():
    return ["png", "jpeg", "jpg", "svg", "ico", "webp", "bin"]

def file_exclusions():
    return [
        ".DS_Store", ".wasm",
        "graph_data.csv", "node-exporter-full_rev31.json",
        "api.json", "full_moon.csv", "models.rs", "grafana.ini",
        "package-lock.json",
        "all_repo_contents.txt",
        "structs_pb2.py",
        "safe.json",
        "repo_summary.txt",
        ".env"
    ]

def file_inclusions():
    return [".gitignore", "Dockerfile", "NOTICE", "certificate_renewal"]

def endings_inclusions():
    return [
        "rs", "sh", "md", "toml", "yaml", "py", "js", "json",
        "production", "html", "env", "yml", "sol", "abi", "csv",
        "txt", "service", "vue", "proto", "sql", "Dockerfile",
        "conf", "ini", "ts"
    ]

@dataclasses.dataclass
class FileData:
    path: str
    contents: str
    lines: list[str]

class AccumFileData:
    def __init__(self):
        self.good_files: list[Path] = []
        self.unknown_files = []
        self.root = Path.cwd().parent

    @classmethod
    def from_config(cls, config):
        return scan_tld(config)

    @classmethod
    def default(cls):
        return scan_tld()

    def relative_path(self, path):
        return path[len(str(self.root))+1:]

    def indexed_files(self):
        indexed = []
        for gf in self.good_files:
            with open(gf, 'r') as file:
                lines = file.readlines()
                contents = "\n".join(lines)
                fd = FileData(str(gf), contents, lines)
                indexed.append(fd)
        return indexed

    def contents_all(self):
        all_content = ""
        for gf in self.good_files:
            with open(gf, 'r') as file:
                contents = file.read()
                if contents.strip() != "":
                    all_content += f"\nTEXT BELOW FROM FILE PATH: {gf}\n{contents}"
        return all_content

    def token_count_all(self):
        content = self.contents_all()
        return count_tokens(content)

    def contents_path(self):
        return [(str(gf), gf.read_text()) for gf in self.good_files]

    def line_counts(self):
        vec = [(str(gf), count_lines(gf)) for gf in self.good_files]
        return sorted(vec, key=lambda x: x[1], reverse=True)

    def token_counts(self):
        vec = [(str(path), count_tokens(contents)) for path, contents in self.contents_path()]
        return sorted(vec, key=lambda x: x[1], reverse=True)


class ScanConfig:
    def __init__(self):
        self.exclude_vue = False
        self.exclude_docs = False
        self.docs_only = False


def process_file(file, accum, file_exclusions, file_inclusions, endings_exclusions, endings_inclusions):
    file_name = file.name
    if file_name in file_exclusions:
        return
    if file_name in file_inclusions:
        accum.good_files.append(file)
        return
    if file.suffix:
        ext = file.suffix[1:]  # Remove the leading dot
        if ext in endings_exclusions:
            return
        if ext in endings_inclusions:
            accum.good_files.append(file)
            return
    accum.unknown_files.append(file)
    
WORKSPACE_PATH = Path.home().joinpath("ai/redgold")


def scan_tld(scan_config=None):

    git_ignore = GitignoreFilter(str(WORKSPACE_PATH))
    current_dir = WORKSPACE_PATH
    # print("Current directory: ", current_dir)
    parent_dir = current_dir.parent

    accum = AccumFileData()
    accum.root = parent_dir

    directory_excl = directory_exclusions()
    file_excl = file_exclusions()
    file_incl = file_inclusions()
    endings_excl = endings_exclusions()
    endings_incl = endings_inclusions()

    directory_inclusions=None

    if scan_config:
        if scan_config.exclude_vue:
            directory_excl.extend(["vue-website", "vue-explorer"])
        if scan_config.exclude_docs:
            directory_excl.append("docs")
        if scan_config.docs_only:
            directory_inclusions = ["docs/content/3.whitepaper"]

    scan_dir(parent_dir, accum, directory_excl, file_excl, file_incl, endings_excl, endings_incl, directory_inclusions)
    return accum


def scan_dir(
        dir_path,
        accum,
        directory_exclusions,
        file_exclusions,
        file_inclusions,
        endings_exclusions,
        endings_inclusions,
        directory_inclusions=None
):
    if dir_path.name in directory_exclusions:
        return
    for entry in sorted(dir_path.iterdir()):
        if entry.is_file():
            skip = False
            if directory_inclusions is not None:
                full_path = str(entry.absolute())
                should_proceed = any([d in full_path for d in directory_inclusions])
                if not should_proceed:
                    skip = True
            if not skip:
                # print("Processing file: ", entry)
                process_file(entry, accum, file_exclusions, file_inclusions, endings_exclusions, endings_inclusions)
        elif entry.is_dir():
            scan_dir(entry, accum, directory_exclusions, file_exclusions, file_inclusions, endings_exclusions, endings_inclusions, directory_inclusions)


def count_lines(file_path):
    with open(file_path, 'r') as file:
        return sum(1 for _ in file)


def count_tokens(text):
    encoding = tiktoken.get_encoding("cl100k_base")
    return len(encoding.encode(text, disallowed_special=()))


def all_repo_contents():
    config = ScanConfig()
    ac = AccumFileData.from_config(config)
    return ac.contents_all()


# Test function
def test_scan_tld():
    config = ScanConfig()
    config.docs_only = True
    ac = AccumFileData.from_config(config)
    # print(f"Token count: {ac.token_count_all()}")
    # lc = ac.line_counts()
    # tlc = sum(count for _, count in lc)
    # print(f"Total line count: {tlc}")
    # print(f"Line counts: {lc}")
    # print(f"Token counts: {ac.token_counts()}")
    # for k in ac.indexed_files():
    #     root = ac.root
    #     print(k.path[len(str(root)):])
    #     print(root)
    with open("all_repo_contents.txt", "w") as f:
        f.write(ac.contents_all())


if __name__ == "__main__":
    test_scan_tld()