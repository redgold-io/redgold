import dataclasses
import tiktoken
from pathlib import Path


def directory_exclusions():
    return [
        ".git", "target", "node_modules", ".idea", "experiments", "venv", ".nuxt", ".output", "dist",
        ".sqlx",
        "ignore-data"
    ]

def endings_exclusions():
    return ["png", "jpeg", "jpg", "svg", "ico", "webp", "bin"]

def file_exclusions():
    return [
        ".DS_Store", ".wasm",
        "graph_data.csv", "node-exporter-full_rev31.json",
        "api.json", "full_moon.csv", "models.rs", "grafana.ini",
        "package-lock.json"
    ]

def file_inclusions():
    return [".env", ".gitignore", "Dockerfile", "NOTICE", "certificate_renewal"]

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
        self.good_files = []
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


def scan_tld(scan_config=None):
    current_dir = Path.cwd()
    print("Current directory: ", current_dir)
    parent_dir = current_dir.parent

    accum = AccumFileData()
    accum.root = parent_dir

    directory_excl = directory_exclusions()
    file_excl = file_exclusions()
    file_incl = file_inclusions()
    endings_excl = endings_exclusions()
    endings_incl = endings_inclusions()

    if scan_config:
        if scan_config.exclude_vue:
            directory_excl.extend(["vue-website", "vue-explorer"])
        if scan_config.exclude_docs:
            directory_excl.append("docs")

    scan_dir(parent_dir, accum, directory_excl, file_excl, file_incl, endings_excl, endings_incl)
    return accum


def scan_dir(dir_path, accum, directory_exclusions, file_exclusions, file_inclusions, endings_exclusions, endings_inclusions):
    if dir_path.name in directory_exclusions:
        return
    for entry in dir_path.iterdir():
        if entry.is_file():
            process_file(entry, accum, file_exclusions, file_inclusions, endings_exclusions, endings_inclusions)
        elif entry.is_dir():
            scan_dir(entry, accum, directory_exclusions, file_exclusions, file_inclusions, endings_exclusions, endings_inclusions)


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
    ac = AccumFileData.from_config(config)
    # print(f"Token count: {ac.token_count_all()}")
    lc = ac.line_counts()
    tlc = sum(count for _, count in lc)
    # print(f"Total line count: {tlc}")
    # print(f"Line counts: {lc}")
    # print(f"Token counts: {ac.token_counts()}")
    for k in ac.indexed_files():
        root = ac.root
        print(k.path[len(str(root)):])
        print(root)


if __name__ == "__main__":
    test_scan_tld()