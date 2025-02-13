import time
from typing import Callable
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
import os
from pathlib import Path
import pathspec

class GitignoreFilter:
    def __init__(self, base_path: str):
        self.base_path = base_path
        self.spec = self._load_gitignore()
        # Common editor backup/temporary file patterns
        self.backup_patterns = pathspec.PathSpec.from_lines(
            pathspec.patterns.GitWildMatchPattern,
            ['*~', '*.swp', '*.swo', '#*#', '.#*']  # Common patterns for vim, emacs, and other editors
        )
        
    def _load_gitignore(self):
        gitignore_path = os.path.join(self.base_path, '.gitignore')
        if not os.path.exists(gitignore_path):
            return None
            
        with open(gitignore_path, 'r') as f:
            gitignore_content = f.read()
        
        return pathspec.PathSpec.from_lines(
            pathspec.patterns.GitWildMatchPattern,
            gitignore_content.splitlines()
        )
    
    def should_ignore(self, path: str) -> bool:
        # Convert absolute path to relative path from base directory
        rel_path = os.path.relpath(path, self.base_path)
        
        # Check for backup/temp files first
        if self.backup_patterns.match_file(rel_path):
            return True
            
        # Then check gitignore patterns if they exist
        if self.spec is not None and self.spec.match_file(rel_path):
            return True
            
        return False

class FileChangeHandler(FileSystemEventHandler):
    def __init__(self, gitignore_filter: GitignoreFilter, handle_event: Callable):
        super().__init__()
        self.gitignore_filter = gitignore_filter
        self.handle_event = handle_event
        
    def _handle_event(self, event, action: str):
        if not event.is_directory and not self.gitignore_filter.should_ignore(event.src_path):
            self.handle_event(event, action)
        
    def on_created(self, event):
        self._handle_event(event, "Created")
            
    def on_modified(self, event):
        self._handle_event(event, "Modified")
            
    def on_deleted(self, event):
        self._handle_event(event, "Deleted")
            
    def on_moved(self, event):
        if not event.is_directory:
            if not (self.gitignore_filter.should_ignore(event.src_path) or 
                   self.gitignore_filter.should_ignore(event.dest_path)):
                # print(f"Moved/Renamed: from {event.src_path} to {event.dest_path}")
                self.handle_event(event, "Moved")
            
def watch_directory(path: str, handle_event: Callable):
    # Expand the ~ to full home directory path
    path = os.path.expanduser(path)
    abs_path = os.path.abspath(path)
    
    if not os.path.exists(abs_path):
        print(f"Directory {abs_path} does not exist!")
        return
        
    gitignore_filter = GitignoreFilter(abs_path)
    event_handler = FileChangeHandler(gitignore_filter, handle_event)
    observer = Observer()
    observer.schedule(event_handler, abs_path, recursive=True)
    observer.start()
    
    print(f"Started watching directory: {abs_path}")
    print("Respecting .gitignore patterns if present")
    print("Press Ctrl+C to stop...")
    
if __name__ == "__main__":

    def handle_event(event, action: str):
        print(f"{action}: {event} {event.src_path} {event.dest_path}")

    watch_directory("~/ai/redgold", handle_event)

    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        # observer.stop()
        print("\nStopped watching directory")
    
    # observer.join()
