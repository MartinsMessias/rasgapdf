import os
import sys
from huggingface_hub import HfApi, create_repo, upload_folder

def main():
    api = HfApi()
    try:
        user_info = api.whoami()
        username = user_info.get("name")
        print(f"✅ Autenticado no Hugging Face como: {username}")
    except Exception as e:
        print(f"❌ Não autenticado no Hugging Face: {e}")
        sys.exit(1)

    repo_id = f"{username}/rasgapdf"
    print(f"🚀 Sincronizando com Hugging Face Hub (Model Repository): {repo_id}...")

    try:
        create_repo(
            repo_id=repo_id,
            repo_type="model",
            exist_ok=True
        )
        print("📦 Fazendo upload dos pacotes, fontes, bindings e binários...")
        upload_folder(
            folder_path="/home/messias_martins/rasgapdf",
            repo_id=repo_id,
            repo_type="model",
            ignore_patterns=["target/*", ".git/*", "*__pycache__*"],
            commit_message="chore: update release packages, bindings and publish script"
        )
        print(f"\n🎉 SUCESSO! Repositório atualizado em:")
        print(f"👉 https://huggingface.co/{repo_id}")
    except Exception as e:
        print(f"❌ Erro ao subir para o Hugging Face: {e}")

if __name__ == "__main__":
    main()
