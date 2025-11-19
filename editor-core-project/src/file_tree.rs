use std::path::{Path, PathBuf};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum FileTreeNode {
    Directory {
        name: String,
        path: PathBuf,
        children: HashMap<String, FileTreeNode>,
        expanded: bool,
    },
    File {
        name: String,
        path: PathBuf,
    },
}

impl FileTreeNode {
    pub fn name(&self) -> &str {
        match self {
            FileTreeNode::Directory { name, .. } => name,
            FileTreeNode::File { name, .. } => name,
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            FileTreeNode::Directory { path, .. } => path,
            FileTreeNode::File { path, .. } => path,
        }
    }

    pub fn is_directory(&self) -> bool {
        matches!(self, FileTreeNode::Directory { .. })
    }

    pub fn is_file(&self) -> bool {
        matches!(self, FileTreeNode::File { .. })
    }

    pub fn is_expanded(&self) -> bool {
        match self {
            FileTreeNode::Directory { expanded, .. } => *expanded,
            FileTreeNode::File { .. } => false,
        }
    }

    pub fn set_expanded(&mut self, expanded: bool) {
        if let FileTreeNode::Directory { expanded: exp, .. } = self {
            *exp = expanded;
        }
    }

    pub fn children(&self) -> Option<&HashMap<String, FileTreeNode>> {
        match self {
            FileTreeNode::Directory { children, .. } => Some(children),
            FileTreeNode::File { .. } => None,
        }
    }

    pub fn children_mut(&mut self) -> Option<&mut HashMap<String, FileTreeNode>> {
        match self {
            FileTreeNode::Directory { children, .. } => Some(children),
            FileTreeNode::File { .. } => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileTree {
    root: FileTreeNode,
}

impl FileTree {
    pub fn new(root_path: PathBuf) -> Result<Self, std::io::Error> {
        let root_name = root_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("root")
            .to_string();

        let root = Self::build_tree(&root_path, &root_name)?;
        Ok(Self { root })
    }

    fn build_tree(path: &Path, name: &str) -> Result<FileTreeNode, std::io::Error> {
        if path.is_dir() {
            let mut children = HashMap::new();
            
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();
                let entry_name = entry_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Skip hidden files and directories (starting with .)
                if entry_name.starts_with('.') {
                    continue;
                }

                let node = if entry_path.is_dir() {
                    Self::build_tree(&entry_path, &entry_name)?
                } else {
                    FileTreeNode::File {
                        name: entry_name,
                        path: entry_path,
                    }
                };

                children.insert(entry_name, node);
            }

            Ok(FileTreeNode::Directory {
                name: name.to_string(),
                path: path.to_path_buf(),
                children,
                expanded: true, // Default to expanded
            })
        } else {
            Ok(FileTreeNode::File {
                name: name.to_string(),
                path: path.to_path_buf(),
            })
        }
    }

    pub fn root(&self) -> &FileTreeNode {
        &self.root
    }

    pub fn root_mut(&mut self) -> &mut FileTreeNode {
        &mut self.root
    }

    pub fn find_node(&self, path: &Path) -> Option<&FileTreeNode> {
        Self::find_node_in_tree(&self.root, path)
    }

    fn find_node_in_tree(node: &FileTreeNode, target_path: &Path) -> Option<&FileTreeNode> {
        if node.path() == target_path {
            return Some(node);
        }

        if let FileTreeNode::Directory { children, .. } = node {
            for child in children.values() {
                if let Some(found) = Self::find_node_in_tree(child, target_path) {
                    return Some(found);
                }
            }
        }

        None
    }

    pub fn find_node_mut(&mut self, path: &Path) -> Option<&mut FileTreeNode> {
        Self::find_node_in_tree_mut(&mut self.root, path)
    }

    fn find_node_in_tree_mut(node: &mut FileTreeNode, target_path: &Path) -> Option<&mut FileTreeNode> {
        if node.path() == target_path {
            return Some(node);
        }

        if let FileTreeNode::Directory { children, .. } = node {
            for child in children.values_mut() {
                if let Some(found) = Self::find_node_in_tree_mut(child, target_path) {
                    return Some(found);
                }
            }
        }

        None
    }

    pub fn refresh(&mut self) -> Result<(), std::io::Error> {
        let root_path = self.root.path().to_path_buf();
        let root_name = self.root.name().to_string();
        
        *self = Self::new(root_path)?;
        Ok(())
    }

    pub fn get_all_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        Self::collect_files(&self.root, &mut files);
        files
    }

    fn collect_files(node: &FileTreeNode, files: &mut Vec<PathBuf>) {
        match node {
            FileTreeNode::File { path, .. } => {
                files.push(path.clone());
            }
            FileTreeNode::Directory { children, .. } => {
                for child in children.values() {
                    Self::collect_files(child, files);
                }
            }
        }
    }

    pub fn get_visible_nodes(&self) -> Vec<&FileTreeNode> {
        let mut nodes = Vec::new();
        Self::collect_visible_nodes(&self.root, &mut nodes);
        nodes
    }

    fn collect_visible_nodes(node: &FileTreeNode, nodes: &mut Vec<&FileTreeNode>) {
        nodes.push(node);

        if let FileTreeNode::Directory { children, expanded, .. } = node {
            if *expanded {
                for child in children.values() {
                    Self::collect_visible_nodes(child, nodes);
                }
            }
        }
    }
}