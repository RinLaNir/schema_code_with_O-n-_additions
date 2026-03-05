use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Ukrainian,
}

/// All translation entries as `(key, english, ukrainian)` triples.
const TRANSLATIONS: &[(&str, &str, &str)] = &[
    // App title
    ("app_title", "Schema Code Benchmarking", "Schema Code Бенчмаркінг"),

    // Main tabs
    ("tab_config", "Configuration", "Конфігурація"),
    ("tab_results", "Results", "Результати"),
    ("tab_console", "Console", "Консоль"),
    ("tab_about", "About", "Про програму"),

    // Status messages
    ("status_ready", "Ready", "Готовий до роботи"),
    ("status_preparing", "Preparing benchmark environment...", "Підготовка середовища для бенчмаркінгу..."),
    ("status_running", "Benchmarking in progress...", "Виконується бенчмаркінг..."),
    ("status_completed", "Benchmarking completed successfully!", "Бенчмаркінг завершено успішно!"),

    // Configuration section
    ("config_title", "Benchmark Configuration", "Налаштування бенчмарків"),
    ("basic_params", "Basic Parameters", "Базові параметри"),
    ("c_value", "C value:", "C значення:"),
    ("particles_to_remove", "Particles to remove:", "Кількість часток для видалення:"),
    ("as_percentage", "As percentage", "Як відсоток"),
    ("llr_value", "LLR value:", "LLR значення:"),
    ("max_iterations", "Maximum iterations:", "Максимальна кількість ітерацій:"),
    ("runs_count", "Number of runs:", "Кількість повторень:"),
    ("warmup_runs", "Warmup runs:", "Прогрівальні запуски:"),
    ("implementation", "Implementation:", "Реалізація:"),
    ("implementation_both", "Both", "Обидві"),
    ("implementation_sequential", "Sequential", "Послідовна"),
    ("implementation_parallel", "Parallel", "Паралельна"),

    // Secret parameters
    ("secret_value", "Secret:", "Секрет:"),
    ("secret_random", "Random", "Випадковий"),
    ("secret_hex", "Hex", "Hex"),
    ("secret_seed", "Seed:", "Seed:"),
    ("secret_seed_hint", "Optional seed value", "Необов'язкове значення seed"),

    // Code parameters
    ("code_params", "Code Parameters", "Параметри коду"),
    ("select_decoders", "Select decoders", "Виберіть декодери"),
    ("select_all", "Select all", "Вибрати всі"),
    ("clear_selection", "Clear selection", "Очистити вибір"),
    ("code_rate", "Code rate:", "Швидкість коду:"),
    ("info_block_size", "Information block size:", "Розмір блоку інформації:"),

    // Output settings
    ("output_settings", "Output Settings", "Налаштування виводу"),
    ("show_details", "Show detailed results for each phase", "Показувати детальні результати для кожної фази"),
    ("save_json", "Save results to JSON", "Зберігати результати в JSON"),
    ("filename_auto", "Enter filename or leave empty for auto", "Введіть ім'я файлу або залиште порожнім"),
    ("verbose_logging", "Verbose logging (detailed phase breakdown)", "Детальне логування (розбивка по фазах)"),
    ("cache_setup", "Cache setup (reuse generator matrix across runs)", "Кешувати setup (повторно використовувати генераторну матрицю)"),

    // Run button & command line
    ("run_benchmark", "▶ Run benchmark", "▶ Запустити бенчмарк"),
    ("stop_benchmark", "⏹ Stop benchmark", "⏹ Зупинити бенчмарк"),
    ("stopping_benchmark", "Stopping after current run...", "Зупинка після поточного запуску..."),
    ("show_command", "Show command line", "Показати командну строку"),
    ("copy_command", "Copy to clipboard", "Копіювати в буфер"),
    ("command_line_label", "Command:", "Команда:"),

    // About tab
    ("about_title", "About Schema Code Benchmarking", "Про Schema Code Benchmarking"),
    ("about_description",
        "Rust implementation of an Additive-Only Secret Sharing (AOS) using CCSDS AR4JA LDPC codes; includes sequential and parallel implementations and a GUI for benchmarking and visualization.",
        "Реалізація Additive-Only Secret Sharing (AOS) на Rust з LDPC-кодами CCSDS AR4JA; містить послідовну й паралельну реалізації та GUI для бенчмаркінгу й візуалізації."),
    ("user_instructions", "User instructions", "Інструкція користувача"),
    ("instruction_1", "1. Configure benchmark parameters on the \"Configuration\" tab.", "1. На вкладці \"Конфігурація\" налаштуйте параметри бенчмарку."),
    ("instruction_2", "2. Click the \"Run benchmark\" button to start testing.", "2. Натисніть кнопку \"Запустити бенчмарк\" для початку тестування."),
    ("instruction_3", "3. After completion, go to the \"Results\" tab to view results.", "3. Після завершення, перейдіть на вкладку \"Результати\" для перегляду результатів."),
    ("instruction_4", "4. You can save results to JSON files for further analysis.", "4. Ви можете зберегти результати в JSON файлах для подальшого аналізу."),
    ("benchmark_params_heading", "Benchmark options", "Опції бенчмарку"),
    ("param_c_desc", "C value: determines the maximum size of random coefficients.", "C значення: визначає максимальний розмір випадкових коефіцієнтів."),
    ("param_runs_desc", "Number of runs: how many times to run each configuration to collect statistics.", "Кількість повторень: скільки разів запускати кожну конфігурацію для збору статистики."),
    ("param_impl_desc", "Implementation: sequential or parallel version of the algorithm.", "Реалізація: послідовна або паралельна версія алгоритму."),
    ("param_decoder_desc", "Decoder type: different methods of decoding LDPC codes.", "Тип декодера: різні методи декодування LDPC кодів."),
    ("param_rate_desc", "Code rate: ratio of message length to codeword length.", "Швидкість коду: відношення довжини повідомлення до довжини кодового слова."),
    ("param_size_desc", "Information block size: size of data blocks in bits.", "Розмір блоку інформації: розмір блоків даних в бітах."),

    // Console tab
    ("console_title", "Console Output", "Консольний вивід"),

    // Results viewer
    ("results_title", "Benchmark Results", "Результати бенчмаркінгу"),
    ("no_results", "No results to display. Run a benchmark first.", "Немає результатів для відображення. Запустіть бенчмарк спочатку."),
    ("tab_summary", "Summary", "Загальні результати"),
    ("tab_details", "Operation Details", "Деталі по операціях"),
    ("tab_phases", "Execution Phases", "Фази виконання"),
    ("tab_visualization", "Visualization", "Візуалізація"),
    ("tab_acceleration", "Acceleration", "Прискорення"),
    ("import_results", "Import Results", "Імпортувати результати"),
    ("import_success", "Results imported successfully", "Результати успішно імпортовано"),
    ("import_error", "Error importing results", "Помилка імпорту результатів"),

    // Summary tab — table columns
    ("total_execution_time", "Total Execution Time", "Загальний час виконання"),
    ("col_implementation", "Implementation", "Імплементація"),
    ("col_block_size", "Block Size", "Розмір блоку"),
    ("col_rate", "Rate", "Швидкість"),
    ("col_decoder", "Decoder", "Декодер"),
    ("col_avg_time", "Avg Time", "Середній час"),
    ("col_min_time", "Min Time", "Мін. час"),
    ("col_max_time", "Max Time", "Макс. час"),
    ("col_success_rate", "Success Rate", "Успішність"),
    ("col_median_time", "Median", "Медіана"),
    ("col_std_dev", "Std Dev", "Стд. відхил."),
    ("col_throughput", "Throughput", "Пропускна зд."),
    ("reset_sort", "Reset sort", "Скинути сортування"),

    // Decoding stats
    ("decoding_stats_title", "Decoding Statistics", "Статистика декодування"),
    ("total_rows", "Total rows:", "Всього рядків:"),
    ("successful_rows", "Successful:", "Успішних:"),
    ("failed_rows", "Failed:", "Невдалих:"),
    ("avg_iterations", "Avg iterations:", "Сер. ітерацій:"),
    ("max_iter_hit", "Hit max iterations:", "Досягли ліміту:"),

    // Parallel metrics
    ("thread_count", "Threads used:", "Використано потоків:"),
    ("parallel_efficiency", "Efficiency:", "Ефективність:"),

    // Visualization tab
    ("chart_title", "Average Execution Time Chart", "Графік середнього часу виконання"),
    ("chart_type_label", "Chart type:", "Тип графіку:"),
    ("chart_type_bar", "Bar Chart", "Стовпчикова діаграма"),
    ("chart_type_line", "Line Chart", "Лінійний графік"),
    ("axis_time_ms", "Time (ms)", "Час (мс)"),
    ("axis_parameters", "Parameters", "Параметри"),
    ("impl_sequential", "Sequential", "Послідовно"),
    ("impl_parallel", "Parallel", "Паралельно"),
    ("legend_sequential", "Sequential", "Послідовна"),
    ("legend_parallel", "Parallel", "Паралельна"),
    ("chart_comparison_title", "Execution Time Comparison", "Порівняння часу виконання"),

    // Acceleration tab
    ("speedup_info_title", "Speedup Information", "Інформація про прискорення"),
    ("label_sequential", "Sequential:", "Послідовно:"),
    ("label_parallel", "Parallel:", "Паралельно:"),
    ("label_speedup", "Speedup:", "Прискорення:"),
    ("acceleration_no_comparison",
        "No comparison data available. Run benchmark with both Sequential and Parallel implementations.",
        "Немає даних для порівняння. Запустіть бенчмарк з обома імплементаціями: Послідовна та Паралельна."),
    ("config_filter", "Configuration filter:", "Фільтр конфігурації:"),
    ("filter_all", "All", "Усі"),
    ("no_data_selected", "No data for selected filter", "Немає даних для обраного фільтра"),
    ("col_config", "Configuration", "Конфігурація"),
    ("col_percent_faster", "% Faster", "% Швидше"),

    // Details tab
    ("setup_time_title", "Setup Time", "Час налаштування"),
    ("deal_time_title", "Deal Time", "Час поділу на частки"),
    ("reconstruct_time_title", "Reconstruct Time", "Час реконструкції секрету"),

    // Phases tab
    ("deal_phases_title", "Deal Process Phases", "Фази процесу поділу на частки"),
    ("reconstruct_phases_title", "Reconstruct Process Phases", "Фази процесу реконструкції секрету"),
    ("col_phase", "Phase", "Фаза"),
    ("col_percent_total", "% of total", "% від загального"),
    ("phase_distribution", "Phase distribution:", "Розподіл фаз:"),
    ("expand_all", "Expand all", "Розгорнути все"),
    ("collapse_all", "Collapse all", "Згорнути все"),

    // Enum labels — info sizes
    ("info_size_k1024", "1024 bits", "1024 біт"),
    ("info_size_k4096", "4096 bits", "4096 біт"),
    ("info_size_k16384", "16384 bits", "16384 біт"),

    // Enum labels — rates
    ("rate_r1_2", "1/2", "1/2"),
    ("rate_r2_3", "2/3", "2/3"),
    ("rate_r4_5", "4/5", "4/5"),

    // Enum labels — decoders
    ("decoder_aminstarf32", "Aminstar (f32)", "Aminstar (f32)"),
    ("decoder_aminstarf64", "Aminstar (f64)", "Aminstar (f64)"),
    ("decoder_phif32", "Phi (f32)", "Phi (f32)"),
    ("decoder_phif64", "Phi (f64)", "Phi (f64)"),
    ("decoder_tanhf32", "Tanh (f32)", "Tanh (f32)"),
    ("decoder_tanhf64", "Tanh (f64)", "Tanh (f64)"),
    ("decoder_minstarappoxbonespartialhardlimit", "MinStar Approx Bones Partial Hard Limit", "MinStar Approx Bones Partial Hard Limit"),
    ("decoder_minstarappoxbonespartialhardlimitmtdeg1clip", "MinStar Approx Bones Partial Hard Limit MtDeg1Clip", "MinStar Approx Bones Partial Hard Limit MtDeg1Clip"),
];

/// Shared translation data — built once, wrapped in Arc for cheap cloning.
struct TranslationData {
    en: HashMap<String, String>,
    uk: HashMap<String, String>,
}

#[derive(Clone)]
pub struct Localization {
    current_language: Language,
    data: Arc<TranslationData>,
}

impl Default for Localization {
    fn default() -> Self {
        let mut en = HashMap::with_capacity(TRANSLATIONS.len());
        let mut uk = HashMap::with_capacity(TRANSLATIONS.len());

        for &(key, english, ukrainian) in TRANSLATIONS {
            en.insert(key.to_string(), english.to_string());
            uk.insert(key.to_string(), ukrainian.to_string());
        }

        Self {
            current_language: Language::Ukrainian,
            data: Arc::new(TranslationData { en, uk }),
        }
    }
}

impl Localization {
    pub fn get(&self, key: &str) -> &str {
        let map = match self.current_language {
            Language::English => &self.data.en,
            Language::Ukrainian => &self.data.uk,
        };
        match map.get(key) {
            Some(text) => text,
            None => "[Unknown key]",
        }
    }

    pub fn set_language(&mut self, language: Language) {
        self.current_language = language;
    }
}