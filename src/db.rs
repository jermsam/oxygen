use rusqlite::{params, Connection, types::ToSql};
/// Raw mono audio clips
use color_eyre::eyre::Result;
use crate::audio_clips::AudioClip;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::io::Cursor;

pub struct Db(Connection);

impl Db {
    pub fn open(path: &str) -> Result<Db> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "page_size", 8192)?;
        conn.pragma_update(None, "user_version", 1)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS audio_clips (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                created_at TEXT NOT NULL,
                sample_rate INTEGER NOT NULL,
                playback_position INTEGER NOT NULL DEFAULT 0,
                samples BLOB NOT NULL
            )",
            [],
        )?;
        Ok(Db(conn))
    }

    pub fn save(&self, audio_clip: &mut AudioClip) -> Result<()> {
      
        // Convert Vec<f32> to bytes
        let samples_blob = f32_vec_to_blob(&audio_clip.samples)?;

        self.0.execute(
            "INSERT OR REPLACE INTO audio_clips (name, created_at, sample_rate, playback_position, samples) VALUES (?, ?, ?, ?, ?)",
            params![
                audio_clip.name,
                audio_clip.created_at.to_string(),
                audio_clip.sample_rate,
                audio_clip.playback_position,
                samples_blob
            ],
        )?;
        if audio_clip.id.is_none() {
            audio_clip.id = Some(self.0.last_insert_rowid() as usize);
        }
        Ok(())
    }
    pub fn load(&self, name: &str) -> Result<AudioClip> {
        let mut stmt = self.0.prepare("SELECT * FROM audio_clips WHERE name = ?")?;
        let mut rows = stmt.query(params![name])?;
        let audio_clip = rows.next()?.map(|row| {
            let id: usize = row.get(0)?;
            let name: String = row.get(1)?;
            let created_at: String = row.get(2)?;
            let sample_rate: u32 = row.get(3)?;
            let playback_position: u32 = row.get(4)?;
            let samples_blob: Vec<u8> = row.get(5)?;
            let samples = blob_to_f32_vec(&samples_blob).unwrap();
            let created_at = created_at.parse().unwrap();
            Ok(AudioClip {
                id: Some(id),
                name,
                created_at,
                samples,
                sample_rate,
                playback_position: playback_position as usize,
            })
        }).unwrap_or_else(|| {
            Err(rusqlite::Error::QueryReturnedNoRows)
        })?;
        
        Ok(audio_clip)
    }   
}

// Helper function to convert Vec<f32> to a blob (Vec<u8>) for storage
fn f32_vec_to_blob(samples: &[f32]) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(samples.len() * std::mem::size_of::<f32>());
    let mut cursor = Cursor::new(&mut bytes);
    
    for &sample in samples {
        cursor.write_f32::<LittleEndian>(sample)?;
    }
    
    Ok(bytes)
}

// Helper function to convert blob back to Vec<f32> when reading from DB
fn blob_to_f32_vec(blob: &[u8]) -> Result<Vec<f32>> {
    let sample_count = blob.len() / std::mem::size_of::<f32>();
    let mut samples = Vec::with_capacity(sample_count);
    
    let mut cursor = Cursor::new(blob);
    for _ in 0..sample_count {
        let sample = cursor.read_f32::<LittleEndian>()?;
        samples.push(sample);
    }
    
    Ok(samples)
}