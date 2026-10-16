#!/usr/bin/env bash
set -e

echo "=========================================================="
echo "🚀 RasgaPDF Polyglot Package Publisher (Cargo, PyPI, npm)"
echo "=========================================================="

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

# 1. CARGO (crates.io)
echo ""
echo "📦 [1/3] Publicando no crates.io (Rust / Cargo)..."
if [ -f ~/.cargo/credentials.toml ] || [ -n "$CARGO_REGISTRY_TOKEN" ]; then
    echo "🔑 Credenciais do Cargo detectadas. Publicando..."
    cargo publish
    echo "✅ Crate rasgapdf publicado com sucesso no crates.io!"
else
    echo "⚠️ Token do crates.io não encontrado."
    echo "👉 Obtenha um token em: https://crates.io/settings/tokens"
    echo "👉 Execute: cargo login <seu_token_aqui>"
fi

# 2. PyPI (pip)
echo ""
echo "📦 [2/3] Publicando no PyPI (Python / pip)..."
cd "$REPO_ROOT/bindings/python"
rm -rf dist/ build/ *.egg-info
python3 -m build
twine check dist/*

if [ -f ~/.pypirc ] || [ -n "$TWINE_PASSWORD" ] || [ -n "$TWINE_TOKEN" ]; then
    echo "🔑 Credenciais do PyPI detectadas. Enviando com twine..."
    twine upload dist/* --non-interactive
    echo "✅ Pacote rasgapdf publicado com sucesso no PyPI!"
else
    echo "⚠️ Token do PyPI não encontrado."
    echo "👉 Obtenha um token em: https://pypi.org/manage/account/token/"
    echo "👉 Para publicar agora, execute:"
    echo "   twine upload bindings/python/dist/* -u __token__ -p <seu_pypi_token>"
fi

# 3. NPM (Node.js)
echo ""
echo "📦 [3/3] Publicando no npm (Node.js)..."
cd "$REPO_ROOT/bindings/nodejs"
if npm whoami >/dev/null 2>&1; then
    echo "🔑 Sessão ativa no npm como $(npm whoami). Publicando..."
    npm publish --access public
    echo "✅ Pacote rasgapdf publicado com sucesso no npm!"
else
    echo "⚠️ Sessão do npm não autenticada."
    echo "👉 Execute: cd bindings/nodejs && npm login"
    echo "👉 E depois: npm publish --access public"
fi

echo ""
echo "=========================================================="
echo "🎉 Verificação de pacotes concluída!"
echo "=========================================================="
