"""Sincroniza a documentacao (Model README) e a pagina estatica do Space no Hugging Face Hub.

Uso:
    python upload_to_hf.py

Requer autenticacao previa (`hf auth login`) ou a variavel de ambiente `HF_TOKEN`.
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

from huggingface_hub import HfApi, create_repo, upload_file, upload_folder, CommitOperationDelete

REPO_ROOT = os.path.dirname(os.path.abspath(__file__))


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

    # 1. Sincroniza Model Card (Apenas README.md no Model repository)
    print(f"\n[1/2] Sincronizando Model Card (apenas README.md): https://huggingface.co/{repo_id} (v{version})...")
    try:
        create_repo(repo_id=repo_id, repo_type="model", exist_ok=True, token=token)
        
        # Remove eventuais arquivos de codigo remanescentes
        existing_model_files = api.list_repo_files(repo_id=repo_id, repo_type="model")
        stray_model_files = [f for f in existing_model_files if f not in {"README.md", ".gitattributes"}]
        if stray_model_files:
            print(f"   Limpando {len(stray_model_files)} arquivos de codigo remanescentes do model repo...")
            api.create_commit(
                repo_id=repo_id,
                repo_type="model",
                operations=[CommitOperationDelete(path_in_repo=f) for f in stray_model_files],
                commit_message=f"chore: remove source code and retain only README (v{version})",
            )

        readme_path = os.path.join(REPO_ROOT, "README.md")
        upload_file(
            path_or_fileobj=readme_path,
            path_in_repo="README.md",
            repo_id=repo_id,
            repo_type="model",
            token=token,
            commit_message=f"docs: release v{version} model card sync",
        )
        print("[OK] Model Card (README.md) atualizado com sucesso!")
    except Exception as e:
        print(f"[ERRO] Erro ao sincronizar Model Card: {e}")
        sys.exit(1)

    # 2. Sincroniza Space (Visualizador estatico WASM)
    space_path = os.path.join(REPO_ROOT, "space_hf")
    if os.path.isdir(space_path):
        print(f"\n[2/2] Sincronizando Space Estatico: https://huggingface.co/spaces/{repo_id}...")
        try:
            create_repo(repo_id=repo_id, repo_type="space", space_sdk="static", exist_ok=True, token=token)

            # Limpa arquivos obsoletos (ex.: Dockerfile, server.py)
            valid_static_files = {
                ".gitattributes",
                "README.md",
                "index.html",
                "rasgapdf.wasm",
                "sample.pdf",
                "llms.txt",
                "llms-full.txt",
                "robots.txt",
                "sitemap.xml",
            }
            existing_space_files = api.list_repo_files(repo_id=repo_id, repo_type="space")
            stray_space_files = [f for f in existing_space_files if f not in valid_static_files]
            if stray_space_files:
                print(f"   Limpando {len(stray_space_files)} arquivos legados do Space...")
                api.create_commit(
                    repo_id=repo_id,
                    repo_type="space",
                    operations=[CommitOperationDelete(path_in_repo=f) for f in stray_space_files],
                    commit_message=f"chore: clean up non-static space files (v{version})",
                )

            upload_folder(
                folder_path=space_path,
                repo_id=repo_id,
                repo_type="space",
                token=token,
                commit_message=f"chore: release v{version} static space web visualizer sync",
            )
            print("[OK] Space Estatico atualizado com sucesso!")
        except Exception as e:
            print(f"[AVISO] Erro ao sincronizar Space: {e}")

    print("\n[OK] Hugging Face sincronizado com sucesso (Model: apenas README | Space: estatico)!")


if __name__ == "__main__":
    main()
