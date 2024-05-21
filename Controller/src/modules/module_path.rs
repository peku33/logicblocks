use itertools::Itertools;
use std::{fmt, iter};

fn item_validate(item: &str) {
    assert!(!item.is_empty(), "item must not be empty");
    assert!(
        item.chars().all(|character| character.is_ascii_lowercase()
            || character.is_ascii_uppercase()
            || character.is_ascii_digit()
            || character == '_'),
        "item must be lowercase, uppercase, digit or underscore"
    );
}

#[derive(Debug)]
pub struct ModulePath {
    items: &'static [&'static str],
}
impl ModulePath {
    pub fn new(items: &'static [&'static str]) -> Self {
        assert!(!items.is_empty(), "items must not be empty");
        items.iter().for_each(|item| item_validate(item));

        Self { items }
    }

    pub fn thread_name(&self) -> String {
        self.items.iter().rev().join(".")
    }
    pub fn file_name(
        &self,
        extension: Option<&str>,
    ) -> String {
        if let Some(extension) = extension {
            item_validate(extension);
        }

        self.items.iter().copied().chain(extension).join(".")
    }
}
impl fmt::Display for ModulePath {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(&self.items.iter().join("."))
    }
}

#[cfg(test)]
mod tests_module_path {
    use super::ModulePath;

    #[test]
    fn test_thread_name_1() {
        let module_path = ModulePath::new(&["this", "is", "module"]);
        assert_eq!(module_path.thread_name(), "module.is.this");
    }

    #[test]
    fn test_file_name_1() {
        let module_path = ModulePath::new(&["this", "is", "module"]);
        assert_eq!(module_path.file_name(None), "this.is.module");
        assert_eq!(module_path.file_name(Some("txt")), "this.is.module.txt");
    }
}

#[derive(Debug)]
pub struct ModulePathName {
    module_path: &'static ModulePath,
    name: String,
}
impl ModulePathName {
    pub fn new(
        module_path: &'static ModulePath,
        name: String,
    ) -> Self {
        item_validate(&name);

        Self { module_path, name }
    }

    pub fn thread_name(&self) -> String {
        format!("{}:{}", &self.name, self.module_path.thread_name())
    }
    pub fn file_name(
        &self,
        extension: Option<&str>,
    ) -> String {
        if let Some(extension) = extension {
            item_validate(extension);
        }

        self.module_path
            .items
            .iter()
            .copied()
            .chain(iter::once(&*self.name))
            .chain(extension)
            .join(".")
    }
}
impl fmt::Display for ModulePathName {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.module_path)
    }
}

#[cfg(test)]
mod tests_module_path_name {
    use super::{ModulePath, ModulePathName};
    use once_cell::sync::Lazy;

    fn module_path() -> &'static ModulePath {
        static MODULE_PATH: Lazy<ModulePath> =
            Lazy::new(|| ModulePath::new(&["this", "is", "module"]));
        &MODULE_PATH
    }

    #[test]
    fn test_thread_name_1() {
        let module_path_name = ModulePathName::new(module_path(), "name".to_string());
        assert_eq!(module_path_name.thread_name(), "name:module.is.this");
    }

    #[test]
    fn test_file_name_1() {
        let module_path_name = ModulePathName::new(module_path(), "name".to_owned());
        assert_eq!(module_path_name.file_name(None), "this.is.module.name");
        assert_eq!(
            module_path_name.file_name(Some("txt")),
            "this.is.module.name.txt"
        );
    }
}

pub trait ModulePathTrait {
    fn thread_name(&self) -> String;
    fn file_name(
        &self,
        extension: Option<&str>,
    ) -> String;
}
impl ModulePathTrait for ModulePath {
    fn thread_name(&self) -> String {
        self.thread_name()
    }
    fn file_name(
        &self,
        extension: Option<&str>,
    ) -> String {
        self.file_name(extension)
    }
}
impl ModulePathTrait for ModulePathName {
    fn thread_name(&self) -> String {
        self.thread_name()
    }
    fn file_name(
        &self,
        extension: Option<&str>,
    ) -> String {
        self.file_name(extension)
    }
}
