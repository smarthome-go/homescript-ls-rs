{
  description = "Homescript LS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }: {
    packages.x86_64-linux.homescript-ls = nixpkgs.legacyPackages.x86_64-linux.rustPlatform.buildRustPackage {
      pname = "homescript-ls";
      version = "1.0.0";
      src = ./.;
      cargoSha256 = "sha256-mNzQjsCm7Jrfukgoutb87fOaH7SxsUA2mYFZBYFT/o4=";
    };

    defaultPackage.x86_64-linux = self.packages.x86_64-linux.homescript-ls;
  };
}
