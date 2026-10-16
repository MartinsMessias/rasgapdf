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
        print("Execute o login antes de subir o Space.")
        sys.exit(1)

    repo_id = f"{username}/rasgapdf"
    print(f"🚀 Criando / Atualizando Hugging Face Space: {repo_id}...")

    try:
        create_repo(
            repo_id=repo_id,
            repo_type="space",
            space_sdk="gradio",
            exist_ok=True
        )
        print("📦 Fazendo upload dos arquivos do Gradio e biblioteca nativa...")
        upload_folder(
            folder_path="/home/messias_martins/rasgapdf/demo_gradio",
            repo_id=repo_id,
            repo_type="space",
            commit_message="feat: deploy RasgaPDF 2D spatial visualizer"
        )
        print(f"\n🎉 SUCESSO! Space disponível em:")
        print(f"👉 https://huggingface.co/spaces/{repo_id}")
    except Exception as e:
        print(f"❌ Erro ao subir para o Hugging Face: {e}")

if __name__ == "__main__":
    main()
