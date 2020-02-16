
//! Reading and editing cargo manifest files.

use std::{
    path::{Path, PathBuf},
    cell::RefCell,
    fs::{self, read_to_string},
    iter::FromIterator,
    fmt::{self, Debug, Formatter},
    str::FromStr,
};
use toml_edit::{
    Document,
    Item, Value, Key,
};
use failure::{Error, format_err};

/// Allows reading and editing a cargo manifest file.
pub struct ManifestFile {
    path: PathBuf,
    toml: RefCell<Document>,
}

impl ManifestFile {
    /// Open a manifest file.
    pub fn new<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>
    {
        let toml = read_to_string(&path)
            .map_err(Error::from)?
            .parse::<Document>()
            .map_err(Error::from)?;
            
        if !toml.as_table().contains_key("dependencies") {
            return Err(format_err!(
                "no dependencies section in manifest at:\n\
                {:?}", path.as_ref()));
        }
        
        Ok(ManifestFile {
            path: path.as_ref().to_owned(),
            toml: RefCell::new(toml),
        })
    }
    
    /// Iterate dependency editors.
    pub fn deps<'s>(&'s self) 
        -> Result<impl Iterator<Item=Dep<'s>> + 's, Error> 
    {
        let doc = self.toml.borrow();
        let deps = doc["dependencies"].as_table_like()
            .ok_or_else(|| format_err!("dependencies is \
                not a table-like at:\n{:?}", self.path))?
            .iter()
            .flat_map(|(key, value)| 
                parse_dep(self, key, value))
            .collect::<Vec<Dep<'s>>>();
        Ok(deps.into_iter())
    }
    
    /// Get a dependency editor by key.
    pub fn dep(&self, key: &DepKey) -> Result<Dep, Error> {
        let doc = self.toml.borrow();
        let dep = doc["dependencies"].as_table_like()
            .ok_or_else(|| format_err!("dependencies is \
                not a table-like at:\n{:?}", self.path))?
            .iter()
            .flat_map(|(key, value)| 
                parse_dep(self, key, value))
            .find(|dep| dep.key == key.key)
            .ok_or_else(|| 
                format_err!("cannot find dependency:\n\
                {:?}", key));
        dep
    }
    
    /// Save to the underlying manifest file.
    pub fn save(&mut self) -> Result<(), Error> {
        let doc = self.toml.borrow();
        let content = doc.to_string();
        fs::write(&self.path, content)
            .map_err(Error::from)
    }
}

fn parse_dep<'m>(
    manifest: &'m ManifestFile,
    key: &str,
    value: &Item,
) -> Option<Dep<'m>> {
    value.as_str()
        .map(|version| Dep {
            manifest,
            key: key.into(),
            package: key.into(),
            source: DepSource::Crates { 
                version: version.to_owned(),
            },
        })
        .or_else(|| value.as_table_like()
            .and_then(|table| {
                let package = table.get("package")
                    .and_then(Item::as_str)
                    .unwrap_or(key)
                    .to_string();
                table.get("version")
                    .and_then(Item::as_str)
                    .map(|version| DepSource::Crates {
                        version: version.to_owned(),
                    })
                    .or_else(|| table.get("path")
                        .and_then(Item::as_str)
                        .map(|path| DepSource::Local {
                            path: path.to_owned(),
                        }))
                    .map(|source| Dep {
                        manifest,
                        key: key.into(),
                        package,
                        source,
                    })
            }))
}

/// Identifying key for a manifest dependency.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DepKey { key: String }

/// A dependency in a manifest file.
pub struct Dep<'a> {
    manifest: &'a ManifestFile,
    key: String,
    package: String,
    source: DepSource,
}

impl<'a> Debug for Dep<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Dep")
            .field("manifest", &self.manifest.path)
            .field("key", &self.key)
            .field("package", &self.package)
            .field("source", &self.source)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DepSource {
    Crates { version: String },
    Local { path: String },
}

impl DepSource {
    pub fn is_crate(&self) -> bool {
        match self {
            &DepSource::Crates { .. } => true,
            _ => false,
        }
    }
    
    pub fn is_local(&self) -> bool {
        match self {
            &DepSource::Local { .. } => true,
            _ => false,
        }
    }
    
    pub fn crate_version(&self) -> Option<&str> {
        match self {
            &DepSource::Crates { ref version } => 
                Some(version.as_str()),
            _ => None,
        }
    }
    
    pub fn local_path(&self) -> Option<&str> {
        match self {
            &DepSource::Local { ref path } => 
                Some(path.as_str()),
            _ => None,
        }
    }
}

impl<'a> Dep<'a> {
    /// Get the package name.
    pub fn package(&self) -> &str { &self.package }
    
    /// Get the dependency key.
    pub fn key(&self) -> DepKey {
        DepKey { key: self.key.clone() }
    }
    
    /// Get the package source
    pub fn source(&self) -> DepSource {
        self.source.clone()
    }
    
    /// Edit the document, changing the package source.
    ///
    /// Changes must still be saved through the underlying
    /// `ManifestFile`.
    pub fn set_source(&mut self, source: DepSource) {
        let mut doc = self.manifest.toml.borrow_mut();
        
        // determine the key/val to insert
        let (key, val) = match source.clone() {
            DepSource::Crates { version } => (
                "version", Value::from(version) ),
            DepSource::Local { path } => (
                "path", Value::from(path) ),
        };
        
        let entry = &mut doc["dependencies"][&self.key];
        let replacement: Item = if entry.is_str() {
            Item::Value(Value::from_iter(vec![(
                &Key::from_str(key).unwrap(), 
                val
            )]))
        } else if entry.is_table() {
            let mut table = entry.as_table()
                .unwrap().clone();
                
            table.remove("version");
            table.remove("path");
            table[key] = Item::Value(val);
            
            Item::Table(table)
        } else if entry.is_inline_table() {
            let mut table = entry.as_inline_table()
                .unwrap().clone();
                
            table.remove("version");
            table.remove("path");
            
            Value::from_iter(vec![(
                &Key::from_str(key).unwrap(), 
                val
            )])
                .as_inline_table_mut()
                .unwrap()
                .merge_into(&mut table);
                
            Item::Value(table.into())
        } else {
            unreachable!()
        };
        
        *entry = replacement.into();
        self.source = source;
    }
}
