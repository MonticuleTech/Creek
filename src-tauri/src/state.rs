use tokio::sync::mpsc;
use crate::modules::pipeline::PipelineCommand;

pub struct AppState {
    pub pipeline_tx: mpsc::Sender<PipelineCommand>,
}
