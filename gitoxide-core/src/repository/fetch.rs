use crate::OutputFormat;
use git::bstr::BString;
use git_repository as git;

pub struct Options {
    pub format: OutputFormat,
    pub dry_run: bool,
    pub remote: Option<String>,
    /// If non-empty, override all ref-specs otherwise configured in the remote
    pub ref_specs: Vec<BString>,
}

pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 1..=2;

pub(crate) mod function {
    use super::Options;
    use crate::OutputFormat;
    use anyhow::bail;
    use git_repository as git;
    use git_repository::prelude::ObjectIdExt;
    use git_repository::refspec::match_group::validate::Fix;
    use git_repository::remote::fetch::Status;

    pub fn fetch(
        repo: git::Repository,
        progress: impl git::Progress,
        mut out: impl std::io::Write,
        err: impl std::io::Write,
        Options {
            format,
            dry_run,
            remote,
            ref_specs,
        }: Options,
    ) -> anyhow::Result<()> {
        if format != OutputFormat::Human {
            bail!("JSON output isn't yet supported for fetching.");
        }

        let mut remote = crate::repository::remote::by_name_or_url(&repo, remote.as_deref())?;
        if !ref_specs.is_empty() {
            remote.replace_refspecs(ref_specs.iter(), git::remote::Direction::Fetch)?;
        }
        let res: git::remote::fetch::Outcome<'_> = remote
            .connect(git::remote::Direction::Fetch, progress)?
            .prepare_fetch(Default::default())?
            .with_dry_run(dry_run)
            .receive(&git::interrupt::IS_INTERRUPTED)?;

        let ref_specs = remote.refspecs(git::remote::Direction::Fetch);
        match res.status {
            Status::NoChange => {
                crate::repository::remote::refs::print_refmap(&repo, ref_specs, res.ref_map, &mut out, err)
            }
            Status::Change { update_refs, .. } | Status::DryRun { update_refs } => {
                print_updates(&repo, update_refs, ref_specs, res.ref_map, &mut out, err)
            }
        }?;
        if dry_run {
            writeln!(out, "DRY-RUN: No ref was updated and no pack was received.").ok();
        }
        Ok(())
    }

    pub(crate) fn print_updates(
        repo: &git::Repository,
        update_refs: git::remote::fetch::refs::update::Outcome,
        refspecs: &[git::refspec::RefSpec],
        mut map: git::remote::fetch::RefMap<'_>,
        mut out: impl std::io::Write,
        mut err: impl std::io::Write,
    ) -> anyhow::Result<()> {
        let mut last_spec_index = usize::MAX;
        let mut updates = update_refs
            .iter_mapping_updates(&map.mappings, refspecs)
            .collect::<Vec<_>>();
        updates.sort_by_key(|t| t.2);
        for (update, mapping, spec, edit) in updates {
            if mapping.spec_index != last_spec_index {
                last_spec_index = mapping.spec_index;
                spec.to_ref().write_to(&mut out)?;
                writeln!(out)?;
            }

            write!(out, "\t")?;
            match &mapping.remote {
                git::remote::fetch::Source::ObjectId(id) => {
                    write!(out, "{}", id.attach(repo).shorten_or_id())?;
                }
                git::remote::fetch::Source::Ref(r) => {
                    crate::repository::remote::refs::print_ref(&mut out, r)?;
                }
            };
            match edit {
                Some(edit) => {
                    writeln!(out, " -> {} [{}]", edit.name, update.mode)
                }
                None => writeln!(out, " (fetch only)"),
            }?;
        }
        if !map.fixes.is_empty() {
            writeln!(
                err,
                "The following destination refs were removed as they didn't start with 'ref/'"
            )?;
            map.fixes.sort_by_key(|f| match f {
                Fix::MappingWithPartialDestinationRemoved { spec, .. } => *spec,
            });
            let mut prev_spec = None;
            for fix in &map.fixes {
                match fix {
                    Fix::MappingWithPartialDestinationRemoved { name, spec } => {
                        if prev_spec.map_or(true, |prev_spec| prev_spec != spec) {
                            prev_spec = spec.into();
                            spec.write_to(&mut err)?;
                            writeln!(err)?;
                        }
                        writeln!(err, "\t{name}")?;
                    }
                }
            }
        }
        if map.remote_refs.len() - map.mappings.len() != 0 {
            writeln!(
                err,
                "server sent {} tips, {} were filtered due to {} refspec(s).",
                map.remote_refs.len(),
                map.remote_refs.len() - map.mappings.len(),
                refspecs.len()
            )?;
        }
        Ok(())
    }
}
