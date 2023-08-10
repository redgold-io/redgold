brew install llvm@12 openssl@3
echo 'export PATH="/usr/local/opt/llvm@12/bin:$PATH"' >> /Users/runner/.bash_profile
echo "$(llvm-config --version)"