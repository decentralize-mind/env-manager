class EnvManager < Formula
  desc "Enterprise-grade secrets management with Web3 security features"
  homepage "https://github.com/decentralize-mind/env-manager"
  
  # Option 1: Build from source (recommended for development)
  url "https://github.com/decentralize-mind/env-manager.git", tag: "v0.1.0"
  
  # Option 2: Use pre-built binary (uncomment for production releases)
  # url "https://github.com/decentralize-mind/env-manager/releases/download/v0.1.0/env-manager-0.1.0-aarch64-apple-darwin.tar.gz"
  # sha256 "REPLACE_WITH_ACTUAL_SHA256_HASH"
  
  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # Test that the binary runs
    system "#{bin}/env-manager", "--help"
    
    # Test vault initialization
    system "#{bin}/env-manager", "vault-init"
  end
end
