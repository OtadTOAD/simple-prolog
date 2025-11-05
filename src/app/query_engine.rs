/// Query Engine for Prolog-like queries
///
/// Supports:
/// - Simple fact queries: animal(X)
/// - Rules: student(X, Y) :- attends(X, Y), enrolled(X)
/// - Pattern generation: phrase(pattern_name, X) to generate all combinations
/// - Conjunction queries: animal(X), action(Y)
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Fact {
    pub predicate: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub head: Fact,
    pub body: Vec<Fact>,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub name: String,
    pub components: Vec<String>,
}

pub struct QueryEngine {
    facts: Vec<Fact>,
    rules: Vec<Rule>,
    patterns: Vec<Pattern>,
    fact_map: HashMap<String, Vec<usize>>,
}

impl QueryEngine {
    pub fn new() -> Self {
        Self {
            facts: Vec::new(),
            rules: Vec::new(),
            patterns: Vec::new(),
            fact_map: HashMap::new(),
        }
    }

    pub fn load_config_file(&mut self, path: &str) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        self.load_config(&content)
    }

    pub fn load_config(&mut self, config: &str) -> Result<(), String> {
        for line in config.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Check if it's a rule (contains :-)
            if line.contains(":-") {
                self.add_rule(line)?;
            }
            // Check if it's a pattern (contains -->)
            else if line.contains("-->") {
                self.add_pattern(line)?;
            }
            // Otherwise skip (could be a comment or empty)
        }

        Ok(())
    }

    pub fn load_facts_from_output(&mut self, prolog_output: &str) {
        self.facts.clear();
        self.fact_map.clear();

        for line in prolog_output.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            if let Some(fact) = self.parse_fact(line) {
                let idx = self.facts.len();
                self.fact_map
                    .entry(fact.predicate.clone())
                    .or_insert_with(Vec::new)
                    .push(idx);
                self.facts.push(fact);
            }
        }
    }

    fn parse_fact(&self, line: &str) -> Option<Fact> {
        let line = line.trim_end_matches('.').trim();
        let open_paren = line.find('(')?;
        let close_paren = line.rfind(')')?;

        let predicate = line[..open_paren].trim().to_string();
        let args_str = line[open_paren + 1..close_paren].trim();

        let args = if args_str.is_empty() {
            vec![]
        } else {
            args_str.split(',').map(|s| s.trim().to_string()).collect()
        };

        Some(Fact { predicate, args })
    }

    pub fn parse_fact_public(&self, line: &str) -> Option<Fact> {
        self.parse_fact(line)
    }

    pub fn add_fact(&mut self, fact: Fact) {
        let idx = self.facts.len();
        self.fact_map
            .entry(fact.predicate.clone())
            .or_insert_with(Vec::new)
            .push(idx);
        self.facts.push(fact);
    }

    pub fn add_rule(&mut self, rule_str: &str) -> Result<(), String> {
        let parts: Vec<&str> = rule_str.split(":-").collect();
        if parts.len() != 2 {
            return Err("Rule must have format: head :- body".to_string());
        }

        let head = self
            .parse_fact(parts[0].trim())
            .ok_or("Invalid head in rule")?;

        let body_parts = self.split_by_top_level_comma(parts[1]);
        let mut body = Vec::new();

        for part in body_parts {
            let fact = self
                .parse_fact(&part)
                .ok_or(format!("Invalid body fact: {}", part))?;
            body.push(fact);
        }

        self.rules.push(Rule { head, body });
        Ok(())
    }

    pub fn add_pattern(&mut self, pattern_str: &str) -> Result<(), String> {
        let parts: Vec<&str> = pattern_str.split("-->").collect();
        if parts.len() != 2 {
            return Err("Pattern must have format: name --> components".to_string());
        }

        let name = parts[0].trim().to_string();
        let components = self.split_by_top_level_comma(parts[1]);

        self.patterns.push(Pattern { name, components });
        Ok(())
    }

    /// Execute a query and return results
    /// Supports:
    /// - Simple queries: "animal(X)"
    /// - Conjunction queries: "animal(X), action(Y)"
    /// - Phrase queries: "phrase(sentence, X)" to generate patterns
    pub fn query(&self, query_str: &str) -> Result<Vec<String>, String> {
        let query_str = query_str.trim_end_matches('.').trim();

        if query_str.starts_with("phrase(") {
            return self.query_phrase(query_str);
        }

        if self.is_conjunction(query_str) {
            return self.query_conjunction(query_str);
        }

        self.query_simple(query_str)
    }

    fn is_conjunction(&self, query_str: &str) -> bool {
        let mut paren_depth = 0;
        for ch in query_str.chars() {
            match ch {
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                ',' if paren_depth == 0 => return true,
                _ => {}
            }
        }
        false
    }

    fn split_by_top_level_comma(&self, s: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut paren_depth = 0;

        for ch in s.chars() {
            match ch {
                '(' => {
                    paren_depth += 1;
                    current.push(ch);
                }
                ')' => {
                    paren_depth -= 1;
                    current.push(ch);
                }
                ',' if paren_depth == 0 => {
                    parts.push(current.trim().to_string());
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            parts.push(current.trim().to_string());
        }

        parts
    }

    fn query_simple(&self, query_str: &str) -> Result<Vec<String>, String> {
        let query_fact = self.parse_fact(query_str).ok_or("Invalid query format")?;

        let mut results = Vec::new();
        let mut seen = HashSet::new();

        // Forward direction: query predicate matches fact predicate
        if let Some(indices) = self.fact_map.get(&query_fact.predicate) {
            for &idx in indices {
                let fact = &self.facts[idx];
                if let Some(bindings) = self.unify(&query_fact.args, &fact.args) {
                    let result = self.format_bindings(&bindings);
                    if seen.insert(result.clone()) {
                        results.push(result);
                    }
                }
            }
        }

        // Backward direction: check if query predicate appears as an argument in facts
        // For example: query "animal(X)" should match fact "bear(animal)"
        // This treats "bear(animal)" as equivalent to "animal(bear)"
        for fact in &self.facts {
            for (arg_idx, arg) in fact.args.iter().enumerate() {
                if arg == &query_fact.predicate {
                    let mut reversed_args = vec![fact.predicate.clone()];

                    for (i, other_arg) in fact.args.iter().enumerate() {
                        if i != arg_idx {
                            reversed_args.push(other_arg.clone());
                        }
                    }

                    if let Some(bindings) = self.unify(&query_fact.args, &reversed_args) {
                        let result = self.format_bindings(&bindings);
                        if seen.insert(result.clone()) {
                            results.push(result);
                        }
                    }
                }
            }
        }

        for rule in &self.rules {
            if rule.head.predicate == query_fact.predicate {
                if let Some(rule_results) = self.evaluate_rule(rule, &query_fact.args) {
                    for result in rule_results {
                        if seen.insert(result.clone()) {
                            results.push(result);
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    fn query_conjunction(&self, query_str: &str) -> Result<Vec<String>, String> {
        let predicates = self.split_by_top_level_comma(query_str);

        let mut all_results = vec![HashMap::new()];

        for pred_str in predicates {
            let query_fact = self
                .parse_fact(&pred_str)
                .ok_or(format!("Invalid predicate: {}", pred_str))?;

            let mut new_results = Vec::new();

            for existing_bindings in &all_results {
                let substituted_args: Vec<String> = query_fact
                    .args
                    .iter()
                    .map(|arg| {
                        if arg
                            .chars()
                            .next()
                            .map(|c| c.is_uppercase())
                            .unwrap_or(false)
                        {
                            existing_bindings
                                .get(arg)
                                .cloned()
                                .unwrap_or_else(|| arg.clone())
                        } else {
                            arg.clone()
                        }
                    })
                    .collect();

                if let Some(indices) = self.fact_map.get(&query_fact.predicate) {
                    for &idx in indices {
                        let fact = &self.facts[idx];
                        if let Some(bindings) = self.unify(&substituted_args, &fact.args) {
                            let mut combined = existing_bindings.clone();
                            combined.extend(bindings);
                            new_results.push(combined);
                        }
                    }
                }
            }

            all_results = new_results;
        }

        let results: Vec<String> = all_results
            .into_iter()
            .map(|b| self.format_bindings(&b))
            .collect();

        Ok(results)
    }

    fn query_phrase(&self, query_str: &str) -> Result<Vec<String>, String> {
        let query_str = query_str.trim_end_matches(')').trim();
        let parts: Vec<&str> = query_str.split('(').collect();
        if parts.len() != 2 {
            return Err("Invalid phrase query format".to_string());
        }

        let args: Vec<&str> = parts[1].split(',').map(|s| s.trim()).collect();
        if args.len() != 2 {
            return Err("phrase/2 expects 2 arguments: phrase(pattern, Variable)".to_string());
        }

        let pattern_name = args[0];
        let var_name = args[1];

        let pattern = self
            .patterns
            .iter()
            .find(|p| p.name == pattern_name)
            .ok_or(format!("Pattern '{}' not defined", pattern_name))?;

        let mut results = Vec::new();
        self.generate_combinations(&pattern.components, 0, &mut Vec::new(), &mut results)?;

        let formatted: Vec<String> = results
            .into_iter()
            .map(|combination| format!("{} = [{}]", var_name, combination.join(", ")))
            .collect();

        Ok(formatted)
    }

    fn generate_combinations(
        &self,
        components: &[String],
        index: usize,
        current: &mut Vec<String>,
        results: &mut Vec<Vec<String>>,
    ) -> Result<(), String> {
        if index >= components.len() {
            results.push(current.clone());
            return Ok(());
        }

        let component = &components[index];

        if let Some(indices) = self.fact_map.get(component) {
            for &idx in indices {
                let fact = &self.facts[idx];
                if fact.args.len() == 1 {
                    current.push(fact.args[0].clone());
                    self.generate_combinations(components, index + 1, current, results)?;
                    current.pop();
                }
            }
        } else {
            return Err(format!("No facts found for component '{}'", component));
        }

        Ok(())
    }

    fn unify(
        &self,
        query_args: &[String],
        fact_args: &[String],
    ) -> Option<HashMap<String, String>> {
        if query_args.len() != fact_args.len() {
            return None;
        }

        let mut bindings = HashMap::new();

        for (q_arg, f_arg) in query_args.iter().zip(fact_args.iter()) {
            if q_arg
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
            {
                if let Some(existing) = bindings.get(q_arg) {
                    if existing != f_arg {
                        return None;
                    }
                } else {
                    bindings.insert(q_arg.clone(), f_arg.clone());
                }
            } else {
                if q_arg != f_arg {
                    return None;
                }
            }
        }

        Some(bindings)
    }

    fn evaluate_rule(&self, rule: &Rule, query_args: &[String]) -> Option<Vec<String>> {
        let head_bindings = self.unify(query_args, &rule.head.args)?;

        let mut all_bindings = vec![head_bindings];

        for body_fact in &rule.body {
            let mut new_bindings = Vec::new();

            for existing in &all_bindings {
                let substituted_args: Vec<String> = body_fact
                    .args
                    .iter()
                    .map(|arg| {
                        if arg
                            .chars()
                            .next()
                            .map(|c| c.is_uppercase())
                            .unwrap_or(false)
                        {
                            existing.get(arg).cloned().unwrap_or_else(|| arg.clone())
                        } else {
                            arg.clone()
                        }
                    })
                    .collect();

                // Forward matching: body_fact predicate matches fact predicate
                if let Some(indices) = self.fact_map.get(&body_fact.predicate) {
                    for &idx in indices {
                        let fact = &self.facts[idx];
                        if let Some(bindings) = self.unify(&substituted_args, &fact.args) {
                            let mut combined = existing.clone();
                            combined.extend(bindings);
                            new_bindings.push(combined);
                        }
                    }
                }

                // Bidirectional matching: check if body_fact predicate appears in fact arguments
                for fact in &self.facts {
                    for (arg_idx, arg) in fact.args.iter().enumerate() {
                        if arg == &body_fact.predicate {
                            // Reverse the fact
                            let mut reversed_args = vec![fact.predicate.clone()];
                            for (i, other_arg) in fact.args.iter().enumerate() {
                                if i != arg_idx {
                                    reversed_args.push(other_arg.clone());
                                }
                            }

                            if let Some(bindings) = self.unify(&substituted_args, &reversed_args) {
                                let mut combined = existing.clone();
                                combined.extend(bindings);
                                new_bindings.push(combined);
                            }
                        }
                    }
                }
            }

            all_bindings = new_bindings;
        }

        if all_bindings.is_empty() {
            None
        } else {
            Some(
                all_bindings
                    .into_iter()
                    .map(|b| self.format_bindings(&b))
                    .collect(),
            )
        }
    }

    fn format_bindings(&self, bindings: &HashMap<String, String>) -> String {
        if bindings.is_empty() {
            "true.".to_string()
        } else {
            let mut pairs: Vec<_> = bindings.iter().collect();
            pairs.sort_by_key(|(k, _)| k.to_string());
            pairs
                .iter()
                .map(|(k, v)| format!("{} = {}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_query() {
        let mut engine = QueryEngine::new();
        engine.load_facts_from_output("animal(bear).\nanimal(deer).\nanimal(owl).");

        let results = engine.query("animal(X)").unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_pattern_generation() {
        let mut engine = QueryEngine::new();
        engine.load_facts_from_output("animal(bear).\nanimal(deer).\naction(chase).\naction(run).");
        engine.add_pattern("sentence --> animal, action").unwrap();

        let results = engine.query("phrase(sentence, X)").unwrap();
        assert_eq!(results.len(), 4);
    }
}
