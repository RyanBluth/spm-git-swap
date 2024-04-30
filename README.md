# SPM-Git-Swap

Tired of waiting and waiting for XCode to resolve SPM packages? Then you've come to the right place. SPM-Git-Swap parses your project folder for `Package.resolved` files and locally clones all dependencies. It then updates your global git config `git config --global --edit` and tells git to use the local repos instead of the remotes. When SPM goes to fetch dependencies it will use the local versions.

## How to Use

1. Install Rust - https://rustup.rs/
2. Run the project and point it to your iOS project folder `cargo run --release install my_ios_project_folder`
3. Wait a while as the repositories are cloned
4. SPM should now use the local versions

### Things to Note

- SPM-Git-Swap will fetch if the repo is already cloned. If you alter your SPM dependencies you will need to run the script again to get the latest changes from each repo.
- Currently, there is no handling for weird states that may occur from killing the install half way through. You can run `cargo run --release wipe` to wipe your caches. This will not update your git config so you will need to either clear that manually `git config --edit --global` or run `cargo run --release install my_ios_project_folder` again.
