#!/usr/bin/env python3
"""
Inventory and categorize all Tauri commands in commands.rs.
Helps track which commands have been audited for security.

Usage:
    python3 scripts/audit_ipc_surface.py [--format json|markdown|csv]
"""

import re
import sys
from pathlib import Path
from typing import List, Tuple, Optional
import json

# Categories based on parameter patterns
CATEGORIES = {
    "path_read": [
        "pdf_path", "file_path", "path",
        "image_path", "source_path",
    ],
    "path_write": [
        "output_path", "destination_path",
    ],
    "id": ["id", "workbook_id", "sheet_id", "invoice_id", "client_id"],
    "string": ["name", "description", "title", "notes"],
    "cloud": ["spreadsheet_id", "database_id", "api_key"],
    "internal": ["db", "state"],
}

class CommandAudit:
    def __init__(self, name: str, line: int):
        self.name = name
        self.line = line
        self.params: List[Tuple[str, str]] = []  # (param_name, param_type)
        self.is_async = False
        self.category = None
        self.risk_level = "Low"

    def add_param(self, name: str, type_str: str):
        self.params.append((name, type_str))
        self._update_category()

    def _update_category(self):
        """Infer category from parameters."""
        param_names = [p[0] for p in self.params]

        if any(p in param_names for p in CATEGORIES["path_read"]):
            self.category = "Read Path"
            self.risk_level = "Critical"
        elif any(p in param_names for p in CATEGORIES["path_write"]):
            self.category = "Write Path"
            self.risk_level = "Critical"
        elif any(p in param_names for p in CATEGORIES["id"]):
            self.category = "Database ID"
            self.risk_level = "Medium"
        elif any(p in param_names for p in CATEGORIES["string"]):
            self.category = "String Input"
            self.risk_level = "Medium"
        elif any(p in param_names for p in CATEGORIES["cloud"]):
            self.category = "Cloud Integration"
            self.risk_level = "Medium"
        else:
            self.category = "Database Query"
            self.risk_level = "Low"

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "line": self.line,
            "category": self.category,
            "risk_level": self.risk_level,
            "is_async": self.is_async,
            "parameters": [{"name": n, "type": t} for n, t in self.params],
        }

    def to_markdown(self) -> str:
        params_str = ", ".join([f"`{n}: {t}`" for n, t in self.params])
        return f"- [ ] `{self.name}({params_str})` — {self.risk_level} ({self.category})"

    def to_csv_row(self) -> str:
        params_str = "|".join([f"{n}:{t}" for n, t in self.params])
        return f"{self.name},{self.category},{self.risk_level},{params_str},{self.line}"

def extract_commands(commands_file: Path) -> List[CommandAudit]:
    """Extract all Tauri commands from commands.rs."""
    content = commands_file.read_text()
    commands = []

    # Split by #[tauri::command]
    command_blocks = re.split(r'#\[tauri::command\]', content)

    for block in command_blocks[1:]:  # Skip the part before the first command
        lines = block.split('\n')

        # Find the function definition
        func_line = None
        is_async = False
        name = None
        line_num = 0

        for i, line in enumerate(lines):
            if 'async fn' in line:
                is_async = True

            # Match function signature
            match = re.search(r'(?:async\s+)?pub\s+(?:async\s+)?fn\s+(\w+)\s*\(', line)
            if match:
                name = match.group(1)
                func_line = i
                break

        if not name:
            continue

        # Extract parameters from the function signature
        # This is a simplified approach; a real parser would be more robust
        param_text = ""
        for i in range(func_line, min(func_line + 20, len(lines))):
            param_text += lines[i]
            if ')' in lines[i] and '->' in lines[i]:
                break

        # Parse parameters
        cmd = CommandAudit(name, line_num)
        cmd.is_async = is_async

        # Extract parameter names and types from the function signature
        # Look for patterns like: name: Type
        param_pattern = r'(\w+):\s*([^,\)]+)'
        for param_match in re.finditer(param_pattern, param_text):
            param_name = param_match.group(1).strip()
            param_type = param_match.group(2).strip()

            # Skip Tauri internal params
            if param_name not in ['db', 'app', 'handle', 'state']:
                cmd.add_param(param_name, param_type)

        commands.append(cmd)

    return sorted(commands, key=lambda c: c.name)

def print_markdown_audit(commands: List[CommandAudit]):
    """Print audit checklist in Markdown format."""
    by_category = {}
    for cmd in commands:
        cat = cmd.category or "Unknown"
        if cat not in by_category:
            by_category[cat] = []
        by_category[cat].append(cmd)

    for category in sorted(by_category.keys()):
        cmds = by_category[category]
        risk_levels = set(c.risk_level for c in cmds)
        print(f"\n### {category} ({len(cmds)} commands)")
        print(f"Risk Levels: {', '.join(sorted(risk_levels))}\n")

        for cmd in sorted(cmds, key=lambda c: c.name):
            print(cmd.to_markdown())

def print_json_audit(commands: List[CommandAudit]):
    """Print audit data as JSON."""
    data = {
        "total": len(commands),
        "by_category": {},
        "commands": [c.to_dict() for c in commands],
    }

    for cmd in commands:
        cat = cmd.category or "Unknown"
        if cat not in data["by_category"]:
            data["by_category"][cat] = 0
        data["by_category"][cat] += 1

    print(json.dumps(data, indent=2))

def print_csv_audit(commands: List[CommandAudit]):
    """Print audit data as CSV."""
    print("Command,Category,Risk,Parameters,Line")
    for cmd in sorted(commands, key=lambda c: c.name):
        print(cmd.to_csv_row())

def main():
    commands_file = Path(__file__).parent.parent / "src-tauri" / "src" / "commands.rs"

    if not commands_file.exists():
        print(f"Error: {commands_file} not found", file=sys.stderr)
        sys.exit(1)

    print(f"Scanning {commands_file}...", file=sys.stderr)
    commands = extract_commands(commands_file)
    print(f"Found {len(commands)} commands\n", file=sys.stderr)

    # Determine output format
    output_format = "markdown"
    if len(sys.argv) > 1 and sys.argv[1].startswith("--format="):
        output_format = sys.argv[1].split("=")[1]
    elif len(sys.argv) > 2 and sys.argv[1] == "--format":
        output_format = sys.argv[2]

    if output_format == "json":
        print_json_audit(commands)
    elif output_format == "csv":
        print_csv_audit(commands)
    else:  # markdown (default)
        print_markdown_audit(commands)

if __name__ == "__main__":
    main()
