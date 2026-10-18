#!/usr/bin/env bash
set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

echo "=========================================================="
echo "⚡ RasgaPDF Dual-Cloud Sync: GitHub + Hugging Face"
echo "=========================================================="

# 1. GitHub (Git SSH)
echo ""
echo "🚀 [1/3] Sincronizando com GitHub (origin main)..."
git push origin main
echo "✅ GitHub sincronizado com sucesso!"

# 2. Hugging Face Model Repository
echo ""
echo "🤗 [2/3] Sincronizando com Hugging Face Hub (Model / Code Repo)..."
python3 upload_to_hf.py

# 3. Hugging Face Space (Live Demo)
echo ""
echo "🌐 [3/3] Sincronizando com Hugging Face Space (Web Demo)..."
python3 -c "
from huggingface_hub import HfApi
api = HfApi()
user = api.whoami().get('name')
repo_id = f'{user}/rasgapdf'
api.upload_folder(
    folder_path='$REPO_ROOT/space_hf',
    repo_id=repo_id,
    repo_type='space',
    commit_message='sync: update space web demo'
)
print('✅ Hugging Face Space sincronizado com sucesso!')
"

echo ""
echo "=========================================================="
echo "🎉 Sincronização completa finalizada com sucesso!"
echo "👉 GitHub:        https://github.com/MartinsMessias/rasgapdf"
echo "👉 HF Model Repo: https://huggingface.co/martinsmessias/rasgapdf"
echo "👉 HF Space Demo: https://huggingface.co/spaces/martinsmessias/rasgapdf"
echo "=========================================================="
