mod acceleration_tab;
mod details_tab;
mod phases_tab;
mod summary_tab;
pub mod table_builder;
mod utils;
mod visualization_tab;

pub use acceleration_tab::AccelerationTab;
pub use details_tab::DetailsTab;
pub use phases_tab::PhasesTab;
pub use summary_tab::SummaryTab;
pub use visualization_tab::VisualizationTab;

#[derive(Clone, PartialEq)]
pub enum ResultsTab {
    Summary,
    Details,
    Phases,
    Visualization,
    Acceleration,
}
