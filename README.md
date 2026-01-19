# restty

A dev-friendly HTTP client built with Rust. Powered by Supabase.

## Features

- **No backend to deploy** - Uses Supabase for auth and history
- **Start immediately** - Download and use, no setup required
- Clean, keyboard-driven interface
- Optional cloud history sync with free account
- Keyboard shortcuts (Cmd+L, Cmd+H, Cmd+Enter)
- Dark mode UI
- Cross-platform (macOS, Linux, Windows)

## Download

Download the latest release for your platform:

**[Download from GitHub Releases →](https://github.com/monirz/restty/releases/latest)**

- macOS (Apple Silicon)
- macOS (Intel)
- Linux
- Windows

## Usage

### Quick Start

```bash
./restty
```

1. Open the app
2. Make HTTP requests immediately (no login required)
3. Optional: Sign up for free to save your request history

### History Sync (Optional)

Create a free account to save and sync your request history across devices:

1. Click "Login" in the app
2. Click "Sign Up" tab
3. Enter your email and password
4. Done! Your history is now synced

## Keyboard Shortcuts

- `Cmd+L` - Focus URL bar
- `Cmd+H` - Toggle history panel (when logged in)
- `Cmd+Enter` - Send request
- `Enter` - Submit login
- Click "Continue without login" - Skip to main app

## Development

### Prerequisites

- Rust 1.70+

### Building from Source

```bash
cargo build --release
```

The binary will be at `target/release/restty`

### Using Your Own Supabase Instance

1. Create a free Supabase project at [supabase.com](https://supabase.com)
2. Run the SQL schema from the original Supabase setup
3. Update `SUPABASE_URL` and `SUPABASE_ANON_KEY` in `src/main.rs`
4. Rebuild: `cargo build --release`

## Architecture

- **Frontend**: Rust + egui (native GUI)
- **Backend**: Supabase (auth + PostgreSQL)
- **Auth**: Supabase Auth (email/password)
- **Database**: Supabase PostgreSQL with Row Level Security

All user data is protected by Supabase's Row Level Security policies. Users can only access their own request history.

## GitHub Download Stats

GitHub automatically tracks download counts for each release. View stats on the [Releases page](https://github.com/monirz/restty/releases).

## License

MIT
