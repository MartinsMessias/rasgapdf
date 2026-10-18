"""Publica o repositorio (codigo + bindings + pacotes) no Hugging Face Hub.

Uso:
    python upload_to_hf.py

Requer autenticacao previa (`hf auth login`) ou a variavel de ambiente
`HF_TOKEN` com um token de escrita.
"""
import os
import sys

from huggingface_hub import HfApi, create_repo, upload_folder

REPO_ROOT = os.path.dirname(os.path.abspath(__file__))

# Nunca publicar: build local, artefatos de SO, overrides de maquina e caches.
IGNORE_PATTERNS = [
    "target/**",
    ".git/**",
    ".cargo/**",          # override local de build (nao versionado)
    "examples/**",        # probes de diagnostico
    "**/__pycache__/**",
    "**/*.pyc",
    "**/node_modules/**",
    "**/bin/**",
    "**/obj/**",
    "build/**",
    "dist/**",
    "*.egg-info/**",
]


def main():
    api = HfApi()
    try:
        username = api.whoami()["name"]
        print(f"✅ Autenticado no Hugging Face como: {username}")
    except Exception as e:
        print(f"❌ Não autenticado no Hugging Face: {e}")
        print("   Rode `hf auth login` ou defina HF_TOKEN com um token de escrita.")
        sys.exit(1)

    repo_id = f"{username}/rasgapdf"
    print(f"🚀 Sincronizando com Hugging Face Hub (Model Repository): {repo_id}...")

    try:
        create_repo(repo_id=repo_id, repo_type="model", exist_ok=True)
        print("📦 Fazendo upload dos pacotes, fontes, bindings e binários...")
        upload_folder(
            folder_path=REPO_ROOT,
            repo_id=repo_id,
            repo_type="model",
            ignore_patterns=IGNORE_PATTERNS,
            commit_message="chore: update release packages, bindings and publish script",
        )
        print("\n🎉 SUCESSO! Repositório atualizado em:")
        print(f"👉 https://huggingface.co/{repo_id}")
    except Exception as e:
        print(f"❌ Erro ao subir para o Hugging Face: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
