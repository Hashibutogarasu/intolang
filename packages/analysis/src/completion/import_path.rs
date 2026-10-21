use ipc::{CompletionItem, CompletionKind};
use moduler::BUILTIN_SCHEME;

use super::context::{CompletionContext, Site};
use super::provider::CompletionProvider;
use crate::workspace::{module_entries, package_aliases};

pub struct ImportPathProvider;

impl CompletionProvider for ImportPathProvider {
    fn candidates(&self, context: &CompletionContext<'_>) -> Vec<CompletionItem> {
        let (Site::Path { typed }, Some(packages_dir)) = (&context.site, context.packages_dir)
        else {
            return Vec::new();
        };
        let aliases = package_aliases(packages_dir);
        let Some((scheme, rest)) = typed.split_once("://") else {
            return std::iter::once(BUILTIN_SCHEME.to_owned())
                .chain(aliases)
                .map(|scheme| context.item(format!("{scheme}://"), CompletionKind::Module))
                .collect();
        };
        let (alias, module) = if scheme == BUILTIN_SCHEME {
            match rest.split_once('/') {
                Some(split) => split,
                None => return folders(context, aliases),
            }
        } else {
            (scheme, rest)
        };
        let directory = module
            .rsplit_once('/')
            .map_or("", |(directory, _)| directory);
        module_entries(packages_dir, alias, directory)
            .into_iter()
            .map(|entry| match entry.directory {
                true => CompletionItem {
                    text: format!("{}/", entry.name),
                    ..context.item(entry.name, CompletionKind::Folder)
                },
                false => context.item(entry.name, CompletionKind::Module),
            })
            .collect()
    }
}

fn folders(context: &CompletionContext<'_>, names: Vec<String>) -> Vec<CompletionItem> {
    names
        .into_iter()
        .map(|name| CompletionItem {
            text: format!("{name}/"),
            ..context.item(name, CompletionKind::Folder)
        })
        .collect()
}
