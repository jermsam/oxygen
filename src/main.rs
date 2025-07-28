mod audio_clips;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use audio_clips::AudioClip;
/// A fictional versioning CLI
#[derive(Debug, Parser)] // requires `derive` feature
#[command(name = "oxygen")]
#[command(about = "A voice journal and audio analysis toolkit for people who want to change the way their voice sounds", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Record an audio clip using the default audio device until ctrl + c is pressed
    Record {
        /// The name of the clip to record. If not provided, the current date and time will be used
        name: Option<String>,
    },
    /// List all the clips in our database
    List {
    },
    /// Play the clip with given name
    #[command(arg_required_else_help = true)]
    Play {
        /// The name of the clip to play
        name: String,
    },
    /// Delete the clip with given name
    #[command(arg_required_else_help = true)]
    Delete {
        /// The name of the clip to delete
        name: String,
    },
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    
    match &cli.command {
        Commands::Record { name } => {
            println!("Record command with name: {}", name.as_ref().map_or("default", |s| s));
            let _name = name.as_ref().unwrap_or(&"default".to_string());
            let _audio_clip = AudioClip::record().unwrap();
            
        }
        Commands::List {} => {
            println!("List command");
        }
        Commands::Play { name } => {
            println!("Play command with name: {}", name);
        }
        Commands::Delete { name } => {
            println!("Delete command with name: {}", name);
        }
    }   
    
    println!("{:?}",cli);
    Ok(())
}
