mod audio_clips;
mod db;

use chrono::Local;
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
    let db = db::Db::open("oxygen.db")?;
    match &cli.command {
        Commands::Record { name } => {
            let name = name.clone().unwrap_or_else(|| Local::now().format("%Y-%m-%d_%H-%M-%S").to_string());
            let mut audio_clip = AudioClip::record(name)?;
            db.save(&mut audio_clip)?;
        }
        Commands::List {} => {
            let audio_clips = db.list()?;
            for audio_clip in audio_clips {
                println!("{} {} {} {} ",  audio_clip.name, audio_clip.created_at, audio_clip.sample_rate, audio_clip.playback_position);
            }
        }
        Commands::Play { name } => {
            let audio_clip = db.load(name)?;
            audio_clip.play()?;
        }
        Commands::Delete { name } => {
            db.delete(name)?;
            println!("Deleted clip '{}' successfully.", name);
        }
    }   
    
    println!("{:?}",cli);
    Ok(())
}
