mod summary_tab;
mod details_tab;
mod phases_tab;
mod visualization_tab;
mod acceleration_tab;
mod utils;
pub mod table_builder;

pub use summary_tab::SummaryTab;
pub use details_tab::DetailsTab;
pub use phases_tab::PhasesTab;
pub use visualization_tab::VisualizationTab;
pub use acceleration_tab::AccelerationTab;

#[derive(Clone, PartialEq)]
pub enum ResultsTab {
    Summary,
    Details,
    Phases,
    Visualization,
    Acceleration,
}