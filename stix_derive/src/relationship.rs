use darling::{Error, FromMeta, Result};
use heck::SnakeCase;
use quote::ToTokens;
use syn::{Ident, Meta, NestedMeta};

#[derive(Clone)]
pub struct RelationshipType(Ident);

impl RelationshipType {
    pub fn name(&self) -> String {
        self.0.to_string().to_snake_case()
    }

    pub fn reverse_name(&self) -> String {
        match self.0.to_string().as_str() {
            "Controls" => "controlled_by".to_string(),
            "DerivedFrom" => "derived".to_string(),
            "Drops" => "dropped_by".to_string(),
            "DuplicateOf" => "duplicated_by".to_string(),
            "LocatedAt" => "location_of".to_string(),
            "OriginatesFrom" => "origin_of".to_string(),
            "SubtechniqueOf" => "parent_technique_of".to_string(),
            other => {
                if let Some(s) = invert_relationship(&other.to_snake_case()) {
                    s
                } else {
                    // This is a panic unless/until specific relationships are identified where
                    // declaration-site renaming is appopriate. That would risk consistency issues
                    // across generated collections, which would in term harm interoperability,
                    // so it's better to add special-cases here for now.
                    panic!(
                        "stix_derive cannot generate inverse relationship name for `{}`",
                        self.0
                    );
                }
            }
        }
    }
}

impl From<Ident> for RelationshipType {
    fn from(value: Ident) -> Self {
        Self(value)
    }
}

impl ToTokens for RelationshipType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens);
    }
}

pub struct Relationship {
    pub(crate) rel: RelationshipType,
    pub(crate) to: Ident,
}

impl FromMeta for Relationship {
    fn from_list(items: &[NestedMeta]) -> Result<Self> {
        if items.is_empty() {
            return Err(Error::too_few_items(2));
        }

        let rel = if let NestedMeta::Meta(Meta::Path(path)) = &items[0] {
            path.get_ident()
                .ok_or_else(|| {
                    Error::custom("Relationship must be identified with an identifier")
                        .with_span(path)
                })
                .map(|ident| RelationshipType::from(ident.clone()))
        } else {
            Err(Error::unsupported_format("unknown"))
        };

        let to = if items.len() == 1 {
            Err(Error::too_few_items(2))
        } else if let NestedMeta::Meta(Meta::Path(path)) = &items[1] {
            path.get_ident()
                .ok_or_else(|| Error::custom("Target type must be an identifier").with_span(path))
        } else {
            Err(Error::unsupported_format("unknown target"))
        };

        if rel.is_err() || to.is_err() {
            let mut errors = vec![];
            if let Err(e) = rel {
                errors.push(e);
            }

            if let Err(e) = to {
                errors.push(e);
            }

            return Err(Error::multiple(errors));
        }

        Ok(Self {
            rel: rel.unwrap().clone(),
            to: to.unwrap().clone(),
        })
    }
}

fn replace_suffix(haystack: &str, needle: &str, replacement: &str) -> String {
    if !haystack.ends_with(needle) {
        haystack.to_string()
    } else {
        haystack
            .chars()
            .take(haystack.len() - needle.len())
            .chain(replacement.chars())
            .collect()
    }
}

fn invert_relationship(name: &str) -> Option<String> {
    if name.ends_with("d_by") {
        return Some(replace_suffix(name, "d_by", "s"));
    }

    for e_requiring_suffix in &["ds", "rs", "ts"] {
        if name.ends_with(e_requiring_suffix) {
            return Some(replace_suffix(name, "s", "ed_by"));
        }
    }

    if name.ends_with("s") {
        return Some(replace_suffix(name, "s", "d_by"));
    }

    None
}