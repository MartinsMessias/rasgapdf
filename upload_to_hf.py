"""Sincroniza o repositório (Model) e o Space interativo no Hugging Face Hub.

Uso:
    python upload_to_hf.py

Requer autenticação prévia (`hf auth login`) ou a variável de ambiente `HF_TOKEN`.
"""
import os
import re
import sys

# Garante suporte a UTF-8 no stdout do Windows
if sys.stdout.encoding != "utf-8":
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass

from huggingface_hub import HfApi, create_repo, upload_folder

REPO_ROOT = os.path.dirname(os.path.abspath(__file__))

# Arquivos/pastas que não devem ser publicados no Model repo
IGNORE_PATTERNS = [
    "target/**",
    ".git/**",
    ".cargo/**",
    "examples/**",
    "**/__pycache__/**",
    "**/*.pyc",
    "**/node_modules/**",
    "**/bin/**",
    "**/obj/**",
    "build/**",
    "dist/**",
    "*.egg-info/**",
    ".github/**",
]


def get_version():
    cargo_path = os.path.join(REPO_ROOT, "Cargo.toml")
    if os.path.exists(cargo_path):
        with open(cargo_path, "r", encoding="utf-8") as f:
            for line in f:
                m = re.match(r'^version\s*=\s*"([^"]+)"', line.strip())
                if m:
                    return m.group(1)
    return "latest"


def main():
    token = os.environ.get("HF_TOKEN")
    api = HfApi(token=token)

    try:
        user_info = api.whoami()
        username = user_info.get("name")
        print(f"[OK] Autenticado no Hugging Face como: {username}")
    except Exception as e:
        print(f"[ERRO] Nao autenticado no Hugging Face: {e}")
        print("   Defina a variavel de ambiente HF_TOKEN ou faca login localmente.")
        sys.exit(1)

    version = get_version()
    repo_id = f"{username}/rasgapdf"

    # 1. Sincroniza Model Repository
    print(f"\n[1/2] Sincronizando Model Repository: https://huggingface.co/{repo_id} (v{version})...")
    try:
        create_repo(repo_id=repo_id, repo_type="model", exist_ok=True, token=token)
        upload_folder(
            folder_path=REPO_ROOT,
            repo_id=repo_id,
            repo_type="model",
            token=token,
            ignore_patterns=IGNORE_PATTERNS,
            commit_message=f"chore: release v{version} automated sync",
        )
        print("[OK] Model repository atualizado com sucesso!")
    except Exception as e:
        print(f"[ERRO] Erro ao sincronizar Model: {e}")
        sys.exit(1)

    # 2. Sincroniza Space (Visualizador interativo)
    space_path = os.path.join(REPO_ROOT, "space_hf")
    if os.path.isdir(space_path):
        print(f"\n[2/2] Sincronizando Space: https://huggingface.co/spaces/{repo_id}...")
        try:
            create_repo(repo_id=repo_id, repo_type="space", space_sdk="static", exist_ok=True, token=token)
            upload_folder(
                folder_path=space_path,
                repo_id=repo_id,
                repo_type="space",
                token=token,
                commit_message=f"chore: release v{version} space web visualizer sync",
            )
            print("[OK] Space atualizado com sucesso!")
        except Exception as e:
            print(f"[AVISO] Erro ao sincronizar Space: {e}")

    print("\n[OK] Hugging Face sincronizado com sucesso!")


if __name__ == "__main__":
    main()
