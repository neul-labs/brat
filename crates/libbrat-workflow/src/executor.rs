//! Workflow executor - creates convoys and tasks from workflow templates.

use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use libbrat_grite::{DependencyType, GriteeClient};

use crate::error::WorkflowError;
use crate::parser::WorkflowParser;
use crate::schema::{WorkflowTemplate, WorkflowType};

/// Result of executing a workflow.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkflowInstance {
    /// Unique instance ID.
    pub instance_id: String,
    /// Workflow name.
    pub workflow_name: String,
    /// Convoy ID created for this instance.
    pub convoy_id: String,
    /// Task IDs created (in execution order).
    pub task_ids: Vec<String>,
    /// Input variables used.
    pub variables: HashMap<String, String>,
    /// Timestamp when executed.
    pub executed_at: String,
}

/// Executor for running workflow templates.
pub struct WorkflowExecutor {
    /// Gritee client for creating convoys/tasks.
    gritee: GriteeClient,
}

impl WorkflowExecutor {
    /// Create a new executor with the given Gritee client.
    pub fn new(gritee: GriteeClient) -> Self {
        Self { gritee }
    }

    /// Execute a workflow template with the given variables.
    pub fn execute(
        &self,
        template: &WorkflowTemplate,
        vars: HashMap<String, String>,
    ) -> Result<WorkflowInstance, WorkflowError> {
        // Validate required inputs
        for (name, spec) in &template.inputs {
            if spec.required && !vars.contains_key(name) {
                if spec.default.is_none() {
                    return Err(WorkflowError::MissingInput(name.clone()));
                }
            }
        }

        // Build complete variables map with defaults
        let mut complete_vars = HashMap::new();
        for (name, spec) in &template.inputs {
            if let Some(value) = vars.get(name) {
                complete_vars.insert(name.clone(), value.clone());
            } else if let Some(ref default) = spec.default {
                complete_vars.insert(name.clone(), default.clone());
            }
        }

        // Generate instance ID
        let instance_id = format!("wf-{}", Uuid::new_v4().to_string().split('-').next().unwrap());

        // Create convoy
        let convoy_title = WorkflowParser::substitute_vars(
            &format!("[{}] {}", template.name, template.description.as_deref().unwrap_or(&template.name)),
            &complete_vars,
        );
        let convoy_body = format!(
            "Workflow instance: {}\nWorkflow: {}\nVariables: {:?}",
            instance_id, template.name, complete_vars
        );

        let convoy = self.gritee.convoy_create(&convoy_title, Some(&convoy_body))?;

        // Create tasks based on workflow type
        let task_ids = match template.workflow_type {
            WorkflowType::Workflow => self.create_sequential_tasks(template, &convoy.convoy_id, &complete_vars, &instance_id)?,
            WorkflowType::Convoy => self.create_parallel_tasks(template, &convoy.convoy_id, &complete_vars, &instance_id)?,
        };

        Ok(WorkflowInstance {
            instance_id,
            workflow_name: template.name.clone(),
            convoy_id: convoy.convoy_id,
            task_ids,
            variables: complete_vars,
            executed_at: Utc::now().to_rfc3339(),
        })
    }

    /// Create tasks for a sequential workflow.
    fn create_sequential_tasks(
        &self,
        template: &WorkflowTemplate,
        convoy_id: &str,
        vars: &HashMap<String, String>,
        instance_id: &str,
    ) -> Result<Vec<String>, WorkflowError> {
        let mut task_ids = Vec::new();
        let mut step_to_task: HashMap<String, (String, String)> = HashMap::new(); // step_id -> (task_id, gritee_issue_id)

        // Topological sort for dependency ordering
        let ordered_steps = self.topological_sort_steps(template)?;

        for step in ordered_steps {
            let title = WorkflowParser::substitute_vars(&step.title, vars);
            let mut body = WorkflowParser::substitute_vars(&step.body, vars);

            // Add workflow metadata to body
            body = format!(
                "{}\n\n---\nWorkflow: {}\nInstance: {}\nStep: {}",
                body, template.name, instance_id, step.id
            );

            let task = self.gritee.task_create(convoy_id, &title, Some(&body))?;

            // Add dependencies using gritee DAG
            for dep_step_id in &step.needs {
                if let Some((_, dep_gritee_issue_id)) = step_to_task.get(dep_step_id) {
                    // This task depends on the dependency task
                    if let Err(e) = self.gritee.task_dep_add(
                        &task.gritee_issue_id,
                        dep_gritee_issue_id,
                        DependencyType::DependsOn,
                    ) {
                        // Log but don't fail - dependency tracking is optional
                        eprintln!(
                            "Warning: Failed to add dependency {} -> {}: {}",
                            task.task_id, dep_step_id, e
                        );
                    }
                }
            }

            step_to_task.insert(step.id.clone(), (task.task_id.clone(), task.gritee_issue_id.clone()));
            task_ids.push(task.task_id);
        }

        Ok(task_ids)
    }

    /// Create tasks for a parallel convoy.
    fn create_parallel_tasks(
        &self,
        template: &WorkflowTemplate,
        convoy_id: &str,
        vars: &HashMap<String, String>,
        instance_id: &str,
    ) -> Result<Vec<String>, WorkflowError> {
        let mut task_ids = Vec::new();
        let mut leg_gritee_issue_ids = Vec::new();

        // Create a task for each leg (all start as queued - parallel execution)
        for leg in &template.legs {
            let title = WorkflowParser::substitute_vars(&leg.title, vars);
            let mut body = WorkflowParser::substitute_vars(&leg.body, vars);

            // Add workflow metadata
            body = format!(
                "{}\n\n---\nWorkflow: {}\nInstance: {}\nLeg: {}",
                body, template.name, instance_id, leg.id
            );

            let task = self.gritee.task_create(convoy_id, &title, Some(&body))?;
            leg_gritee_issue_ids.push(task.gritee_issue_id.clone());
            task_ids.push(task.task_id);
        }

        // Create synthesis task if defined
        if let Some(ref synthesis) = template.synthesis {
            let title = WorkflowParser::substitute_vars(&synthesis.title, vars);
            let mut body = WorkflowParser::substitute_vars(&synthesis.body, vars);

            // Add workflow metadata
            body = format!(
                "{}\n\n---\nWorkflow: {}\nInstance: {}\nSynthesis: true",
                body, template.name, instance_id
            );

            let task = self.gritee.task_create(convoy_id, &title, Some(&body))?;

            // Add dependencies from synthesis to all legs using gritee DAG
            for leg_issue_id in &leg_gritee_issue_ids {
                if let Err(e) = self.gritee.task_dep_add(
                    &task.gritee_issue_id,
                    leg_issue_id,
                    DependencyType::DependsOn,
                ) {
                    // Log but don't fail - dependency tracking is optional
                    eprintln!(
                        "Warning: Failed to add synthesis dependency: {}",
                        e
                    );
                }
            }

            task_ids.push(task.task_id);
        }

        Ok(task_ids)
    }

    /// Topological sort of workflow steps based on dependencies.
    fn topological_sort_steps<'a>(
        &self,
        template: &'a WorkflowTemplate,
    ) -> Result<Vec<&'a crate::schema::StepSpec>, WorkflowError> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();

        // Build step map for quick lookup
        let step_map: HashMap<&str, &crate::schema::StepSpec> = template
            .steps
            .iter()
            .map(|s| (s.id.as_str(), s))
            .collect();

        // Visit each step
        for step in &template.steps {
            if !visited.contains(&step.id) {
                self.visit_step(
                    &step.id,
                    &step_map,
                    &mut visited,
                    &mut temp_visited,
                    &mut result,
                )?;
            }
        }

        Ok(result)
    }

    /// Helper for topological sort - visits a step and its dependencies.
    fn visit_step<'a>(
        &self,
        step_id: &str,
        step_map: &HashMap<&str, &'a crate::schema::StepSpec>,
        visited: &mut std::collections::HashSet<String>,
        temp_visited: &mut std::collections::HashSet<String>,
        result: &mut Vec<&'a crate::schema::StepSpec>,
    ) -> Result<(), WorkflowError> {
        if temp_visited.contains(step_id) {
            return Err(WorkflowError::CircularDependency);
        }
        if visited.contains(step_id) {
            return Ok(());
        }

        temp_visited.insert(step_id.to_string());

        let step = step_map
            .get(step_id)
            .ok_or_else(|| WorkflowError::UnknownStep(step_id.to_string()))?;

        // Visit dependencies first
        for dep in &step.needs {
            self.visit_step(dep, step_map, visited, temp_visited, result)?;
        }

        temp_visited.remove(step_id);
        visited.insert(step_id.to_string());
        result.push(step);

        Ok(())
    }
}
