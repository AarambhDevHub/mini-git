# Mini Git - A Git Implementation in Rust 🦀

A complete, educational implementation of Git version control system written in Rust. Mini Git demonstrates the core concepts and internal workings of Git, including object storage, branching, merging, and distributed workflow operations.

## 🌟 Features

### Core Git Operations
- ✅ **Repository Management**: `init`, `clone` (local repositories)
- ✅ **Staging & Committing**: `add`, `commit`, `status`
- ✅ **History & Logging**: `log`, `diff`
- ✅ **Branching**: `branch`, `checkout`, `merge`
- ✅ **Remote Operations**: `remote`, `push`, `pull` (local repositories)
- ✅ **Stashing**: `stash push/pop/list/show/drop/clear`

### Advanced Features
- 🗄️ **Object Store**: Git-compatible object storage with compression
- 🌳 **Tree Structure**: Proper Git tree and blob object management
- 🔀 **Three-way Merge**: Intelligent merge conflict detection
- 📦 **Index Management**: Staging area implementation
- 🎯 **Hash-based Integrity**: SHA-1 content addressing
- 🔄 **Distributed Workflow**: Multi-repository synchronization

## 🚀 Quick Start

### Prerequisites
- Rust 1.70 or higher
- Cargo (comes with Rust)

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd mini_git

# Build the project
cargo build --release

# The executable will be at ./target/release/mini_git
```

### Basic Usage

```bash
# Initialize a new repository
./target/release/mini_git init

# Add files to staging area
echo "Hello, World!" > hello.txt
./target/release/mini_git add .

# Commit changes
./target/release/mini_git commit -m "Initial commit" --author "Your Name <your.email@example.com>"

# View history
./target/release/mini_git log

# Check status
./target/release/mini_git status
```

## 📚 Comprehensive Examples

### Basic Workflow
```bash
# Create a new project
mkdir my_project
cd my_project
mini_git init

# Create and track files
echo "# My Project" > README.md
echo "fn main() { println!(\"Hello!\"); }" > main.rs
mini_git add .
mini_git commit -m "Initial project setup"

# Check the project status
mini_git status
mini_git log
```

### Branch Management
```bash
# Create and switch to a new branch
mini_git branch feature-auth
mini_git checkout feature-auth

# Work on the feature
echo "Authentication module" > auth.rs
mini_git add auth.rs
mini_git commit -m "Add authentication module"

# Switch back to main and merge
mini_git checkout main
mini_git merge feature-auth

# Clean up
mini_git branch feature-auth --delete
```

### Distributed Development (Local)
```bash
# Create a central repository
mkdir central_repo
cd central_repo
mini_git init
echo "Project started" > README.md
mini_git add . && mini_git commit -m "Initial commit"

# Developer A clones and contributes
cd ..
mini_git clone central_repo developer_a
cd developer_a
echo "Feature A" > feature_a.txt
mini_git add . && mini_git commit -m "Add Feature A"
mini_git push origin main

# Developer B clones, pulls latest, and contributes
cd ..
mini_git clone central_repo developer_b
cd developer_b
mini_git pull origin main  # Gets Feature A
echo "Feature B" > feature_b.txt
mini_git add . && mini_git commit -m "Add Feature B"
mini_git push origin main

# Developer A pulls latest changes
cd ../developer_a
mini_git pull origin main
ls  # See both features
```

### Stash Workflow
```bash
# Make some changes
echo "Work in progress..." >> important_file.txt

# Need to switch branches quickly? Stash your work
mini_git stash push -m "WIP: updating important file"

# Switch branches, do other work...
mini_git checkout other-branch
# ... do work ...

# Come back and restore your changes
mini_git checkout main
mini_git stash pop  # Restores and removes from stash

# Or keep the stash and just apply it
mini_git stash list
mini_git stash show
mini_git stash apply  # Applies but keeps in stash
```

### Remote Management
```bash
# Add remotes for different purposes
mini_git remote add origin ../main_repo
mini_git remote add backup /path/to/backup/repo
mini_git remote add fork ../forked_repo

# List remotes with URLs
mini_git remote -v

# Change remote URLs
mini_git remote set-url origin ../new_location

# Remove a remote
mini_git remote remove backup
```

## 🏗️ Architecture

### Object Store
Mini Git implements Git's object storage model:

```
.mini_git/
├── objects/           # Object database
│   ├── 12/           # First 2 chars of hash
│   │   └── 3456789...# Remaining hash (zlib compressed)
│   └── ab/
│       └── cdef123...
├── refs/             # Reference storage
│   ├── heads/        # Branch references
│   │   ├── main
│   │   └── feature
│   └── remotes/      # Remote tracking branches
│       └── origin/
│           └── main
├── index             # Staging area
├── HEAD              # Current branch pointer
└── config            # Repository configuration
```

### Object Types
1. **Blob**: File content storage
2. **Tree**: Directory structure and file metadata
3. **Commit**: Snapshot with metadata and parent references

### Key Components

#### Object Store (`src/object_store.rs`)
- Hash-based content addressing using SHA-1
- Zlib compression for efficient storage
- JSON serialization for object metadata

#### Commands (`src/commands/`)
- Modular command implementation
- Each Git command as a separate module
- Consistent error handling and user feedback

#### Repository Management (`src/utils.rs`)
- Repository detection and initialization
- Index management and manipulation
- Branch and reference utilities

## 📁 Project Structure

```
mini_git/
├── src/
│   ├── main.rs              # CLI interface and argument parsing
│   ├── lib.rs               # Core types and structures
│   ├── object_store.rs      # Git object storage implementation
│   ├── utils.rs             # Repository utilities and helpers
│   └── commands/            # Git command implementations
│       ├── mod.rs           # Command module exports
│       ├── init.rs          # Repository initialization
│       ├── add.rs           # Staging area management
│       ├── commit.rs        # Commit creation
│       ├── status.rs        # Working directory status
│       ├── log.rs           # Commit history
│       ├── branch.rs        # Branch management
│       ├── checkout.rs      # Branch switching and file restoration
│       ├── merge.rs         # Three-way merge implementation
│       ├── diff.rs          # File difference calculation
│       ├── clone.rs         # Repository cloning
│       ├── push.rs          # Publishing changes
│       ├── pull.rs          # Fetching and merging changes
│       ├── remote.rs        # Remote repository management
│       └── stash.rs         # Temporary change storage
├── Cargo.toml               # Project configuration and dependencies
└── README.md               # This file
```

## 🧪 Testing

### Automated Testing
Run the comprehensive test suite:

```bash
# Make the test script executable
chmod +x test_minigit.sh

# Run all tests
./test_minigit.sh
```

The test suite covers:
- Basic repository operations
- Branching and merging
- Stashing functionality
- Clone and remote operations
- Distributed workflows
- Error handling
- Object store integrity

### Manual Testing
```bash
# Build and test basic functionality
cargo build --release

# Test basic operations
mkdir test_repo && cd test_repo
../target/release/mini_git init
echo "test" > file.txt
../target/release/mini_git add .
../target/release/mini_git commit -m "Test commit"
../target/release/mini_git log
```

## 🎯 Educational Goals

This project demonstrates:

1. **Git Internals**: How Git stores and manages data
2. **Content-Addressable Storage**: Hash-based data integrity
3. **Directed Acyclic Graph**: Commit history representation
4. **Three-Way Merge**: Conflict resolution algorithms
5. **Distributed Version Control**: Multi-repository workflows
6. **Rust Programming**: Systems programming in Rust

## 🔧 Dependencies

```toml
[dependencies]
sha1 = "0.10"           # SHA-1 hashing
serde = "1.0"           # Serialization
serde_json = "1.0"      # JSON support
chrono = "0.4"          # Date/time handling
clap = "4.0"            # Command-line parsing
walkdir = "2.3"         # Directory traversal
flate2 = "1.0"          # Zlib compression
```

## 🚧 Limitations & Design Decisions

### Local-Only Focus
Mini Git focuses on **local repository operations** for educational clarity:
- ✅ Clone, push, pull work between local repositories
- ❌ Network protocols (HTTP, SSH, Git protocol) not implemented
- 🎯 Demonstrates core Git concepts without network complexity

### Simplified Features
Some features are simplified for learning purposes:
- Basic diff algorithm (not Myers algorithm)
- Simplified merge conflict resolution
- JSON object serialization (instead of Git's custom format)
- No delta compression (for object storage clarity)

### Production Considerations
For production use, you would need:
- Network protocol implementation
- Advanced merge algorithms
- Performance optimizations
- Garbage collection
- Hook system
- Submodule support

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass (`./test_minigit.sh`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## 📖 Learning Resources

To understand the concepts implemented in Mini Git:

- [Pro Git Book](https://git-scm.com/book) - Official Git documentation
- [Git Internals](https://git-scm.com/book/en/v2/Git-Internals-Plumbing-and-Porcelain) - How Git works internally
- [Building Git](http://shop.oreilly.com/product/0636920041771.do) - Step-by-step Git implementation

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Git community for the excellent design and documentation
- Rust community for the amazing ecosystem
- Educational resources that inspired this implementation

***

**Made with ❤️ and 🦀 Rust ❤️ by [Aarambh Dev Hub](https://youtube.com/@aarambhdevhub)**

*Mini Git is an educational project designed to demonstrate Git's internal workings. For production use, please use the official Git implementation.*
