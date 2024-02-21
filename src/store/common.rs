use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{DDNode, NodeID, VarID},
    },
    misc::hash_select::{HashMap, HashSet},
};

impl DDManager {
    /// Loads a BDD from a Nodelist (containing all nodes from a BDD) into the DDManager.
    ///
    /// # Panics
    /// Only allowed on empty DDManagers. If called on a non-empty DDManager, this function will
    /// panic!
    pub fn load_bdd_from_nodelist(
        mut self,
        nodes: HashMap<NodeID, (VarID, NodeID, NodeID)>,
        varorder: Vec<usize>,
        roots: Vec<NodeID>,
        terminals: (NodeID, NodeID),
    ) -> (DDManager, Vec<NodeID>) {
        assert!(
            self.nodes.is_empty(),
            "load_bdd_from_nodelist is only allowed on empty DDManagers."
        );

        // Prepare DDManager
        self.var2level = varorder.clone();

        // Create nodes in DDManager (bottom up)
        let mut new_ids: HashMap<NodeID, NodeID> = HashMap::default();
        new_ids.insert(terminals.0, self.one());
        new_ids.insert(terminals.1, self.zero());

        let layer_to_nodes: HashMap<usize, HashSet<NodeID>> = nodes
            .iter()
            .map(|(n, (v, _, _))| (varorder[v.0], n))
            .fold(HashMap::default(), |mut layer_to_nodes, (l, n)| {
                if let Some(nodes) = layer_to_nodes.get_mut(&l) {
                    nodes.insert(*n);
                } else {
                    let mut nodes = HashSet::default();
                    nodes.insert(*n);
                    layer_to_nodes.insert(l, nodes);
                }
                layer_to_nodes
            });

        let mut layers = varorder;
        layers.sort();
        layers.reverse();
        layers
            .iter()
            .flat_map(|layer| layer_to_nodes.get(layer).unwrap())
            .filter(|node_id| **node_id != terminals.0 && **node_id != terminals.1)
            .for_each(|node_id| {
                let node = nodes.get(node_id).unwrap();
                let new_id = self.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: node.0,
                    low: *new_ids.get(&node.1).unwrap(),
                    high: *new_ids.get(&node.2).unwrap(),
                });
                new_ids.insert(*node_id, new_id);
            });

        // Convert root ids
        let roots: Vec<NodeID> = roots.iter().map(|r| *new_ids.get(r).unwrap()).collect();

        (self, roots)
    }
}
