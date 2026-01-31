use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "zap")]
#[command(about = "⚡ Dead simple E2EE file transfers from your terminal", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    /// Disable TUI (use simple progress bars instead)
    #[arg(long, global = true)]
    pub no_tui: bool,
    
    /// Custom port (default: 9999)
    #[arg(long, short = 'p', global = true)]
    pub port: Option<u16>,
    
    /// Verbose output
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Send a file or directory
    Send {
        /// File or directory to send (or read from stdin if omitted)
        path: Option<PathBuf>,
        
        /// Custom code instead of generating one
        #[arg(long, short = 'c')]
        code: Option<String>,
        
        /// Number of words in generated code (default: 3)
        #[arg(long, short = 'w', default_value = "3")]
        words: usize,
    },
    
    /// Receive a file or directory
    Receive {
        /// Transfer code from sender
        code: String,
        
        /// Output path (or write to stdout if omitted)
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
        
        /// Resume a previous transfer
        #[arg(long, short = 'r')]
        resume: bool,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
