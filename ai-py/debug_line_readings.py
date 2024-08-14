with open("repo_reader.py", "r") as f:
    lines = f.readlines()
    for i, l in enumerate(lines):
        print(f"{i+1}: {l}")
        print("-"*10)