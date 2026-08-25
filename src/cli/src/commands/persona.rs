use anyhow::Result;
use kitsu_core::identity::IdentityStore;
use std::path::Path;

use crate::app::PersonaAction;

pub fn execute(current_dir: &Path, action: Option<PersonaAction>) -> Result<()> {
    let mut store = IdentityStore::load(current_dir);
    match action {
        Some(PersonaAction::Add {
            id,
            name,
            email,
            global,
        }) => {
            let mut i = kitsu_core::identity::Identity {
                id,
                name,
                email,
                public_key: None,
                private_key: None,
            };
            i.generate_keys();
            store.identities.push(i);
            store.save(current_dir, global)?;
        }
        Some(PersonaAction::List) => {
            for i in &store.identities {
                println!("  {} - {} <{}>", i.id, i.name, i.email);
            }
        }
        Some(PersonaAction::Use { id, global }) => {
            store.active_id = id;
            store.save(current_dir, global)?;
        }
        Some(PersonaAction::Edit {
            id,
            name,
            email,
            global,
        }) => {
            if let Some(identity) = store.identities.iter_mut().find(|i| i.id == id) {
                if let Some(n) = name {
                    identity.name = n;
                }
                if let Some(e) = email {
                    identity.email = e;
                }
                store.save(current_dir, global)?;
                println!("Persona '{}' updated.", id);
            } else {
                println!("Persona '{}' not found.", id);
            }
        }
        Some(PersonaAction::Github {
            username,
            id,
            global,
        }) => {
            let persona_id = id.unwrap_or_else(|| username.clone());
            let mut i = kitsu_core::identity::Identity {
                id: persona_id.clone(),
                name: username.clone(),
                email: format!("{}@users.noreply.github.com", username),
                public_key: None,
                private_key: None,
            };
            i.generate_keys();
            store.identities.push(i);
            store.save(current_dir, global)?;
            println!(
                "Persona '{}' created from GitHub user '{}'.",
                persona_id, username
            );
        }
        Some(PersonaAction::Keys) => {
            let a = store.active_id.clone();
            if let Some(id) = store.identities.iter_mut().find(|i| i.id == a) {
                id.generate_keys();
                store.save(current_dir, false)?;
                println!("Keys regenerated for persona '{}'.", a);
            }
        }
        None => {
            let a = store.get_active();
            println!("{} <{}>", a.name, a.email);
        }
    }
    Ok(())
}
