//! The sample provider: the module over fixed test data, for the development of frontends and for tests.
//!
//! Pattern: Repository (a second adapter behind the same port, as `fs-tasks-sample` is for planning) – [`SampleRepo`] is
//! `MeRepo` + `ProfileRepo` over [`sample_profiles`], in memory, nothing written, nothing read from a disk, no clock, no
//! randomness. It says it is sample data ([`PersonRepo::is_sample`]), so a host never shows it as real. Deviation: none. It is
//! only the DATA that differs: the rights are not here, the service puts the same `PersonRules` before it as before a store.

use async_trait::async_trait;
use fs_objects::{ObjectId, Result};

use crate::sample_data::{sample_anchor, sample_profiles};
use crate::{Anchor, Created, MeRepo, PersonRepo, Profile, ProfileRepo};

/// The sample data as a repository: "me" is Amara Zolenko, the profiles are the eleven of [`sample_profiles`].
pub struct SampleRepo {
    anchor: Anchor,
    /// In the order of the stores: by name (case-insensitive), ties by id.
    profiles: Vec<Profile>,
}

impl SampleRepo {
    /// The sample repository (always the same).
    #[must_use]
    pub fn new() -> Self {
        let mut profiles = sample_profiles();
        profiles.sort_by_cached_key(|p| (p.card.display_name().to_lowercase(), p.id));
        Self {
            anchor: sample_anchor(),
            profiles,
        }
    }
}

impl Default for SampleRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MeRepo for SampleRepo {
    async fn me(&self) -> Result<Option<Anchor>> {
        Ok(Some(self.anchor.clone()))
    }

    /// "Me" is there already: the sample anchor comes back, nothing is made and nothing is written.
    async fn create_me_as(&self, _anchor: Anchor) -> Result<Created> {
        Ok(Created {
            anchor: self.anchor.clone(),
            new: false,
        })
    }
}

#[async_trait]
impl ProfileRepo for SampleRepo {
    async fn page(&self, offset: usize, limit: usize) -> Result<Vec<Profile>> {
        Ok(self
            .profiles
            .iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect())
    }

    async fn get(&self, id: ObjectId) -> Result<Option<Profile>> {
        Ok(self.profiles.iter().find(|p| p.id == id).cloned())
    }

    async fn count(&self) -> Result<usize> {
        Ok(self.profiles.len())
    }
}

impl PersonRepo for SampleRepo {
    fn is_sample(&self) -> bool {
        true
    }
}
