#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `pane-shell/tree/model.ts`: the Dockview-style layout
//! tree with split and group nodes. Composes helpers already ported into this host. Only
//! `botInsights` calls this.

use serde_json::{Value, json};

/// Orientations of a split node (Orientation).
pub const ORIENTATIONS: [&str; 2] = ["row", "column"];

/// Standing choices about a tab strip (TabStripMode).
pub const TAB_STRIP_MODES: [&str; 2] = ["always", "never"];

/// Positions where a dragged pane may land (DropPosition).
pub const DROP_POSITIONS: [&str; 5] = ["center", "left", "right", "top", "bottom"];

/// Edges of the root layout (RootEdge).
pub const ROOT_EDGES: [&str; 4] = ["left", "right", "top", "bottom"];

/// Default relative weight of a split child.
pub const DEFAULT_SPLIT_WEIGHT: f64 = 1.0;

/// Smallest valid weight for a split child.
pub const MIN_SPLIT_WEIGHT: f64 = 0.01;

/// Build a group node (group).
pub fn make_group(panes: &[String], id: &str, active: Option<&str>, minimized: bool) -> Value {
    let active = active
        .filter(|value| panes.iter().any(|pane| pane == value))
        .or_else(|| panes.first().map(String::as_str))
        .unwrap_or("");
    json!({"type": "group", "id": id, "panes": panes, "active": active, "minimized": minimized})
}

/// Build a split node (split).
pub fn make_split(children: &[Value], orientation: &str, id: &str) -> Value {
    let weights = children.iter().map(|_| DEFAULT_SPLIT_WEIGHT).collect::<Vec<_>>();
    json!({"type": "split", "id": id, "orientation": orientation, "children": children, "weights": weights})
}

/// Whether a value has the shape of a layout node (isLayoutNode).
pub fn is_layout_node(value: &Value) -> bool {
    match value["type"].as_str().unwrap_or("") {
        "group" => value["panes"].is_array(),
        "split" => value["children"].is_array() && ORIENTATIONS.contains(&value["orientation"].as_str().unwrap_or("")),
        _ => false,
    }
}

/// Find a group by id (findGroup).
pub fn find_group(node: &Value, group_id: &str) -> Value {
    if node["type"].as_str() == Some("group") {
        return if node["id"].as_str() == Some(group_id) { node.clone() } else { Value::Null };
    }
    node["children"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(|child| {
            let found = find_group(child, group_id);
            (!found.is_null()).then_some(found)
        })
        .unwrap_or(Value::Null)
}

/// Find the group holding a pane (findGroupOfPane).
pub fn find_group_of_pane(node: &Value, pane_id: &str) -> Value {
    if node["type"].as_str() == Some("group") {
        let holds =
            node["panes"].as_array().is_some_and(|panes| panes.iter().any(|pane| pane.as_str() == Some(pane_id)));
        return if holds { node.clone() } else { Value::Null };
    }
    node["children"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(|child| {
            let found = find_group_of_pane(child, pane_id);
            (!found.is_null()).then_some(found)
        })
        .unwrap_or(Value::Null)
}

/// All pane ids in a tree (allPaneIds).
pub fn all_pane_ids(node: &Value) -> Vec<String> {
    if node["type"].as_str() == Some("group") {
        return node["panes"]
            .as_array()
            .map_or(Vec::new(), |panes| panes.iter().filter_map(Value::as_str).map(str::to_owned).collect());
    }
    node["children"].as_array().map(|children| children.iter().flat_map(all_pane_ids).collect()).unwrap_or_default()
}

/// Ids of all group nodes in a tree (groupLeafIds).
pub fn group_leaf_ids(node: &Value) -> Vec<String> {
    if node["type"].as_str() == Some("group") {
        return node["id"].as_str().map_or(Vec::new(), |id| vec![id.to_owned()]);
    }
    node["children"].as_array().map(|children| children.iter().flat_map(group_leaf_ids).collect()).unwrap_or_default()
}

/// The split whose direct child has the given id (findParentSplit).
pub fn find_parent_split(node: &Value, child_id: &str) -> Value {
    if node["type"].as_str() != Some("split") {
        return Value::Null;
    }
    let direct = node["children"]
        .as_array()
        .is_some_and(|children| children.iter().any(|child| child["id"].as_str() == Some(child_id)));
    if direct {
        return node.clone();
    }
    node["children"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(|child| {
            let found = find_parent_split(child, child_id);
            (!found.is_null()).then_some(found)
        })
        .unwrap_or(Value::Null)
}

/// Stable order with anchor panes first (anchorsFirst).
pub fn anchors_first(panes: &[String], anchors: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for anchor in anchors {
        if let Some(pane) = panes.iter().find(|pane| *pane == anchor) {
            if !out.contains(pane) {
                out.push(pane.clone());
            }
        }
    }
    for pane in panes {
        if !out.contains(pane) {
            out.push(pane.clone());
        }
    }
    out
}

/// Map a root edge to a drop position (dropPositionFromEdge).
pub fn drop_position_from_edge(edge: &str) -> &'static str {
    match edge {
        "left" => "left",
        "right" => "right",
        "top" => "top",
        "bottom" => "bottom",
        _ => "center",
    }
}

/// Whether a tab strip is visible in a group (resolveTabStripVisible).
pub fn resolve_tab_strip_visible(group: &Value, pane_count: usize) -> bool {
    if group["minimized"] == true {
        return true;
    }
    match group["tabStrip"].as_str().unwrap_or("auto") {
        "always" => true,
        "never" => false,
        _ => pane_count > 1,
    }
}

/// Whether a weight vector matches its children (validWeights).
pub fn valid_weights(weights: &[f64], children: usize) -> bool {
    weights.len() == children
        && !weights.is_empty()
        && weights.iter().all(|weight| weight.is_finite() && *weight >= MIN_SPLIT_WEIGHT)
}

/// Equal weights for a number of children (normalizeWeights).
pub fn normalize_weights(weights: &[f64], children: usize) -> Vec<f64> {
    if valid_weights(weights, children) {
        weights.to_vec()
    } else if children == 0 {
        Vec::new()
    } else {
        vec![DEFAULT_SPLIT_WEIGHT / children as f64; children]
    }
}

/// Number of panes in a tree (paneCount).
pub fn pane_count(node: &Value) -> usize {
    all_pane_ids(node).len()
}

/// Nesting depth of a tree (treeDepth).
pub fn tree_depth(node: &Value) -> usize {
    if node["type"].as_str() == Some("group") {
        return 1;
    }
    node["children"].as_array().map(|children| 1 + children.iter().map(tree_depth).max().unwrap_or(0)).unwrap_or(1)
}

/// Canonicalize a tree: no empty groups, no single-child or repeated splits (normalize).
pub fn normalize(node: &Value) -> Value {
    if node["type"].as_str() == Some("group") {
        let panes = node["panes"].as_array().cloned().unwrap_or_default();
        if panes.is_empty() {
            return Value::Null;
        }
        let mut out = node.clone();
        let active = node["active"].as_str().unwrap_or("");
        if !panes.iter().any(|pane| pane.as_str() == Some(active)) {
            out["active"] = panes[0].clone();
        }
        return out;
    }
    let orientation = node["orientation"].as_str().unwrap_or("row");
    let children: Vec<Value> = node["children"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(|child| {
                    let normalized = normalize(child);
                    (!normalized.is_null()).then_some(normalized)
                })
                .collect()
        })
        .unwrap_or_default();
    if children.is_empty() {
        return Value::Null;
    }
    if children.len() == 1 {
        return children.into_iter().next().unwrap_or(Value::Null);
    }
    let mut flattened = Vec::new();
    for child in children {
        if child["type"].as_str() == Some("split") && child["orientation"].as_str() == Some(orientation) {
            if let Some(grandchildren) = child["children"].as_array() {
                flattened.extend(grandchildren.iter().cloned());
                continue;
            }
        }
        flattened.push(child);
    }
    let weights = normalize_weights(&[], flattened.len());
    json!({"type":"split","id":node["id"],"orientation":orientation,"children":flattened,"weights":weights})
}

/// Remove a pane from a tree (removePane).
pub fn remove_pane(node: &Value, pane_id: &str) -> Value {
    if node["type"].as_str() == Some("group") {
        let panes: Vec<Value> = node["panes"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|pane| pane.as_str() != Some(pane_id))
            .collect();
        if panes.is_empty() {
            return Value::Null;
        }
        let mut out = node.clone();
        out["panes"] = Value::Array(panes.clone());
        if node["active"].as_str() == Some(pane_id) {
            out["active"] = panes[0].clone();
        }
        return out;
    }
    let children: Vec<Value> = node["children"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|child| {
            let next = remove_pane(&child, pane_id);
            (!next.is_null()).then_some(next)
        })
        .collect();
    if children.is_empty() {
        return Value::Null;
    }
    if children.len() == 1 {
        return children.into_iter().next().unwrap_or(Value::Null);
    }
    let mut out = node.clone();
    out["children"] = Value::Array(children.clone());
    out["weights"] = json!(normalize_weights(&[], children.len()));
    out
}

/// Set the active pane of one group (setActivePane).
pub fn set_active_pane(node: &Value, group_id: &str, pane_id: &str) -> Value {
    if node["type"].as_str() == Some("group") {
        let mut out = node.clone();
        if node["id"].as_str() == Some(group_id)
            && node["panes"].as_array().is_some_and(|panes| panes.iter().any(|pane| pane.as_str() == Some(pane_id)))
        {
            out["active"] = Value::String(pane_id.to_owned());
        }
        return out;
    }
    let mut out = node.clone();
    if let Some(children) = node["children"].as_array() {
        out["children"] =
            Value::Array(children.iter().map(|child| set_active_pane(child, group_id, pane_id)).collect());
    }
    out
}

/// Set the minimized state of one group (setGroupMinimized).
pub fn set_group_minimized(node: &Value, group_id: &str, minimized: bool) -> Value {
    if node["type"].as_str() == Some("group") {
        let mut out = node.clone();
        if node["id"].as_str() == Some(group_id) {
            out["minimized"] = Value::Bool(minimized);
        }
        return out;
    }
    let mut out = node.clone();
    if let Some(children) = node["children"].as_array() {
        out["children"] =
            Value::Array(children.iter().map(|child| set_group_minimized(child, group_id, minimized)).collect());
    }
    out
}

/// Mirror a tree horizontally (mirrorTreeHorizontal).
pub fn mirror_tree_horizontal(node: &Value) -> Value {
    if node["type"].as_str() == Some("group") {
        return node.clone();
    }
    let mut out = node.clone();
    if let Some(children) = node["children"].as_array() {
        let reversed: Vec<Value> = children.iter().rev().map(mirror_tree_horizontal).collect();
        out["children"] = Value::Array(reversed);
    }
    out
}

/// Fill in missing ids, orientation and weights of a persisted tree (migratePersistedTree).
pub fn migrate_persisted_tree(node: &Value) -> Value {
    if node["type"].as_str() == Some("group") {
        let mut out = node.clone();
        let panes = node["panes"].as_array().cloned().unwrap_or_default();
        if out["id"].as_str().unwrap_or("").is_empty() {
            out["id"] = Value::String("g-migrated".to_owned());
        }
        if out["active"].as_str().unwrap_or("").is_empty() {
            out["active"] = panes.first().cloned().unwrap_or(Value::Null);
        }
        return out;
    }
    let mut out = node.clone();
    let children: Vec<Value> = node["children"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|child| migrate_persisted_tree(&child))
        .collect();
    if out["id"].as_str().unwrap_or("").is_empty() {
        out["id"] = Value::String("s-migrated".to_owned());
    }
    if !ORIENTATIONS.contains(&out["orientation"].as_str().unwrap_or("")) {
        out["orientation"] = Value::String("row".to_owned());
    }
    let weights: Vec<f64> =
        out["weights"].as_array().map_or(Vec::new(), |values| values.iter().filter_map(Value::as_f64).collect());
    out["children"] = Value::Array(children.clone());
    out["weights"] = json!(normalize_weights(&weights, children.len()));
    out
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(panes: usize, groups: usize, depth: usize, normalized: bool) -> Vec<(&'static str, String)> {
    vec![
        ("panes", count_label(panes)),
        ("groups", count_label(groups)),
        ("depth", count_label(depth)),
        ("normalized", if normalized { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn building_and_queries() {
        let group = make_group(&["chat".to_owned(), "files".to_owned()], "g1", Some("files"), false);
        assert_eq!(group["active"], "files");
        assert!(is_layout_node(&group));
        assert!(!is_layout_node(&json!({"type":"group"})));
        let split = make_split(&[group.clone(), make_group(&["tree".to_owned()], "g2", None, true)], "row", "s1");
        assert!(is_layout_node(&split));
        assert_eq!(all_pane_ids(&split), vec!["chat", "files", "tree"]);
        assert_eq!(group_leaf_ids(&split), vec!["g1", "g2"]);
        assert_eq!(find_group(&split, "g1")["id"], "g1");
        assert_eq!(find_group_of_pane(&split, "tree")["id"], "g2");
        assert_eq!(find_parent_split(&split, "g2")["id"], "s1");
        assert_eq!(pane_count(&split), 3);
        assert_eq!(tree_depth(&split), 2);
    }

    #[test]
    fn anchors_and_tabs() {
        assert_eq!(
            anchors_first(&["a".to_owned(), "b".to_owned(), "c".to_owned()], &["c".to_owned()]),
            vec!["c", "a", "b"]
        );
        assert_eq!(drop_position_from_edge("top"), "top");
        assert_eq!(drop_position_from_edge("middle"), "center");
        let group = make_group(&["a".to_owned(), "b".to_owned()], "g", None, false);
        assert!(resolve_tab_strip_visible(&group, 2));
        assert!(!resolve_tab_strip_visible(&group, 1));
        let mut never = group.clone();
        never["tabStrip"] = Value::String("never".to_owned());
        assert!(!resolve_tab_strip_visible(&never, 3));
        let always = json!({"tabStrip":"always","minimized":false});
        assert!(resolve_tab_strip_visible(&always, 0));
    }

    #[test]
    fn weights_and_normalize() {
        assert!(valid_weights(&[1.0, 2.0], 2));
        assert!(!valid_weights(&[1.0], 2));
        assert_eq!(normalize_weights(&[], 2), vec![0.5, 0.5]);
        let empty = make_group(&[], "g0", None, false);
        assert_eq!(normalize(&empty), Value::Null);
        let inner = make_split(&[make_group(&["a".to_owned()], "a", None, false)], "row", "inner");
        let outer = make_split(&[inner, make_group(&["b".to_owned()], "b", None, false)], "row", "outer");
        let normalized = normalize(&outer);
        assert_eq!(normalized["children"].as_array().map_or(0, Vec::len), 2);
        assert_eq!(normalized["weights"].as_array().map_or(0, Vec::len), 2);
    }

    #[test]
    fn edits_and_migration() {
        let group = make_group(&["a".to_owned(), "b".to_owned()], "g1", Some("b"), false);
        let split = make_split(&[group, make_group(&["c".to_owned()], "g2", None, false)], "row", "s1");
        let removed = remove_pane(&split, "b");
        assert_eq!(all_pane_ids(&removed), vec!["a", "c"]);
        let active = set_active_pane(&split, "g1", "a");
        assert_eq!(find_group(&active, "g1")["active"], "a");
        let minimized = set_group_minimized(&split, "g2", true);
        assert_eq!(find_group(&minimized, "g2")["minimized"], true);
        assert_eq!(pane_count(&mirror_tree_horizontal(&split)), 3);
        let migrated = migrate_persisted_tree(&json!({"type":"group","panes":["a"]}));
        assert_eq!(migrated["active"], "a");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 2, 2, true).len(), 4);
    }
}
