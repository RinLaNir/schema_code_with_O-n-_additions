use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Ukrainian,
}

#[derive(Clone)]
pub struct Localization {
    current_language: Language,
    translations: HashMap<String, HashMap<Language, String>>,
}

impl Default for Localization {
    fn default() -> Self {
        let mut translations = HashMap::new();
        
        // Add all translations
        add_translation(&mut translations, "app_title", 
            "Schema Code Benchmarking", 
            "Schema Code Бенчмаркінг");
        
        // Main tabs
        add_translation(&mut translations, "tab_config", 
            "Configuration", 
            "Конфігурація");
        add_translation(&mut translations, "tab_results", 
            "Results", 
            "Результати");
        add_translation(&mut translations, "tab_console", 
            "Console", 
            "Консоль");
        add_translation(&mut translations, "tab_about", 
            "About", 
            "Про програму");
        
        // Status messages
        add_translation(&mut translations, "status_ready", 
            "Ready", 
            "Готовий до роботи");
        add_translation(&mut translations, "status_preparing", 
            "Preparing benchmark environment...", 
            "Підготовка середовища для бенчмаркінгу...");
        add_translation(&mut translations, "status_running", 
            "Benchmarking in progress...", 
            "Виконується бенчмаркінг...");
        add_translation(&mut translations, "status_completed", 
            "Benchmarking completed successfully!", 
            "Бенчмаркінг завершено успішно!");
        
        // Configuration section
        add_translation(&mut translations, "config_title", 
            "Benchmark Configuration", 
            "Налаштування бенчмарків");
        add_translation(&mut translations, "basic_params", 
            "Basic Parameters", 
            "Базові параметри");
        add_translation(&mut translations, "c_value", 
            "C value:", 
            "C значення:");
        add_translation(&mut translations, "particles_to_remove", 
            "Particles to remove:", 
            "Кількість часток для видалення:");
        add_translation(&mut translations, "as_percentage", 
            "As percentage", 
            "Як відсоток");
        add_translation(&mut translations, "llr_value", 
            "LLR value:", 
            "LLR значення:");
        add_translation(&mut translations, "max_iterations", 
            "Maximum iterations:", 
            "Максимальна кількість ітерацій:");
        add_translation(&mut translations, "runs_count", 
            "Number of runs:", 
            "Кількість повторень:");
        add_translation(&mut translations, "warmup_runs", 
            "Warmup runs:", 
            "Прогрівальні запуски:");
        add_translation(&mut translations, "implementation", 
            "Implementation:", 
            "Реалізація:");
        add_translation(&mut translations, "implementation_both", 
            "Both", 
            "Обидві");
        add_translation(&mut translations, "implementation_sequential", 
            "Sequential", 
            "Послідовна");
        add_translation(&mut translations, "implementation_parallel", 
            "Parallel", 
            "Паралельна");
        
        // Secret parameters
        add_translation(&mut translations, "secret_value", 
            "Secret:", 
            "Секрет:");
        add_translation(&mut translations, "secret_random", 
            "Random", 
            "Випадковий");
        add_translation(&mut translations, "secret_hex", 
            "Hex", 
            "Hex");
        add_translation(&mut translations, "secret_seed", 
            "Seed:", 
            "Seed:");
        add_translation(&mut translations, "secret_seed_hint", 
            "Optional seed value", 
            "Необов'язкове значення seed");
        
        // Code parameters section
        add_translation(&mut translations, "code_params", 
            "Code Parameters", 
            "Параметри коду");
        add_translation(&mut translations, "select_decoders", 
            "Select decoders", 
            "Виберіть декодери");
        add_translation(&mut translations, "select_all", 
            "Select all", 
            "Вибрати всі");
        add_translation(&mut translations, "clear_selection", 
            "Clear selection", 
            "Очистити вибір");
        add_translation(&mut translations, "code_rate", 
            "Code rate:", 
            "Швидкість коду:");
        add_translation(&mut translations, "info_block_size", 
            "Information block size:", 
            "Розмір блоку інформації:");
        
        // Output settings section
        add_translation(&mut translations, "output_settings", 
            "Output Settings", 
            "Налаштування виводу");
        add_translation(&mut translations, "show_details", 
            "Show detailed results for each phase", 
            "Показувати детальні результати для кожної фази");
        add_translation(&mut translations, "save_json", 
            "Save results to JSON", 
            "Зберігати результати в JSON");
        add_translation(&mut translations, "filename_auto", 
            "Enter filename or leave empty for auto", 
            "Введіть ім'я файлу або залиште порожнім");
        add_translation(&mut translations, "verbose_logging", 
            "Verbose logging (detailed phase breakdown)", 
            "Детальне логування (розбивка по фазах)");
        
        // Run button & command line
        add_translation(&mut translations, "run_benchmark", 
            "▶ Run benchmark", 
            "▶ Запустити бенчмарк");
        add_translation(&mut translations, "stop_benchmark", 
            "⏹ Stop benchmark", 
            "⏹ Зупинити бенчмарк");
        add_translation(&mut translations, "stopping_benchmark", 
            "Stopping after current run...", 
            "Зупинка після поточного запуску...");
        add_translation(&mut translations, "benchmark_stopped", 
            "Benchmark stopped by user", 
            "Бенчмарк зупинено користувачем");
        add_translation(&mut translations, "show_command", 
            "Show command line", 
            "Показати командну строку");
        add_translation(&mut translations, "copy_command", 
            "Copy to clipboard", 
            "Копіювати в буфер");
        add_translation(&mut translations, "command_copied", 
            "Copied!", 
            "Скопійовано!");
        add_translation(&mut translations, "command_line_label", 
            "Command:", 
            "Команда:");
        
        // About tab
        add_translation(&mut translations, "about_title", 
            "About Schema Code Benchmarking", 
            "Про Schema Code Benchmarking");
        add_translation(&mut translations, "about_description", 
            "This program is designed for benchmarking various secret sharing scheme implementations using error correction codes.", 
            "Ця програма призначена для бенчмаркінгу різних реалізацій секретного розподілу з використанням кодів корекції помилок.");
        add_translation(&mut translations, "user_instructions", 
            "User Instructions", 
            "Інструкція користувача");
        add_translation(&mut translations, "instruction_1", 
            "1. Configure benchmark parameters on the \"Configuration\" tab.", 
            "1. На вкладці \"Конфігурація\" налаштуйте параметри бенчмарку.");
        add_translation(&mut translations, "instruction_2", 
            "2. Click the \"Run benchmark\" button to start testing.", 
            "2. Натисніть кнопку \"Запустити бенчмарк\" для початку тестування.");
        add_translation(&mut translations, "instruction_3", 
            "3. After completion, go to the \"Results\" tab to view results.", 
            "3. Після завершення, перейдіть на вкладку \"Результати\" для перегляду результатів.");
        add_translation(&mut translations, "instruction_4", 
            "4. You can save results to CSV files for further analysis.", 
            "4. Ви можете зберегти результати в CSV файлах для подальшого аналізу.");
        add_translation(&mut translations, "benchmark_params_heading", 
            "Benchmark Parameters", 
            "Параметри бенчмарку");
        add_translation(&mut translations, "param_c_desc", 
            "C value: determines the number of random coefficients.", 
            "C значення: визначає кількість випадкових коефіцієнтів.");
        add_translation(&mut translations, "param_runs_desc", 
            "Number of runs: how many times to run each configuration to collect statistics.", 
            "Кількість повторень: скільки разів запускати кожну конфігурацію для збору статистики.");
        add_translation(&mut translations, "param_impl_desc", 
            "Implementation: sequential or parallel version of the algorithm.", 
            "Реалізація: послідовна або паралельна версія алгоритму.");
        add_translation(&mut translations, "param_decoder_desc", 
            "Decoder type: different methods of decoding LDPC codes.", 
            "Тип декодера: різні методи декодування LDPC кодів.");
        add_translation(&mut translations, "param_rate_desc", 
            "Code rate: ratio of message length to codeword length.", 
            "Швидкість коду: відношення довжини повідомлення до довжини кодового слова.");
        add_translation(&mut translations, "param_size_desc", 
            "Information block size: size of data blocks in bits.", 
            "Розмір блоку інформації: розмір блоків даних в бітах.");
        
        // Language selector
        add_translation(&mut translations, "language", 
            "Language:", 
            "Мова:");
        add_translation(&mut translations, "lang_en", 
            "English", 
            "Англійська");
        add_translation(&mut translations, "lang_uk", 
            "Ukrainian", 
            "Українська");

        // Console tab
        add_translation(&mut translations, "console_title", 
            "Console Output", 
            "Консольний вивід");

        // Results viewer
        add_translation(&mut translations, "results_title", 
            "Benchmark Results", 
            "Результати бенчмаркінгу");
        add_translation(&mut translations, "no_results", 
            "No results to display. Run a benchmark first.", 
            "Немає результатів для відображення. Запустіть бенчмарк спочатку.");
        add_translation(&mut translations, "tab_summary", 
            "Summary", 
            "Загальні результати");
        add_translation(&mut translations, "tab_details", 
            "Operation Details", 
            "Деталі по операціях");
        add_translation(&mut translations, "tab_phases", 
            "Execution Phases", 
            "Фази виконання");

        // Summary tab
        add_translation(&mut translations, "total_execution_time", 
            "Total Execution Time", 
            "Загальний час виконання");
        add_translation(&mut translations, "col_implementation", 
            "Implementation", 
            "Імплементація");
        add_translation(&mut translations, "col_block_size", 
            "Block Size", 
            "Розмір блоку");
        add_translation(&mut translations, "col_rate", 
            "Rate", 
            "Швидкість");
        add_translation(&mut translations, "col_decoder", 
            "Decoder", 
            "Декодер");
        add_translation(&mut translations, "col_avg_time", 
            "Avg Time", 
            "Середній час");
        add_translation(&mut translations, "col_min_time", 
            "Min Time", 
            "Мін. час");
        add_translation(&mut translations, "col_max_time", 
            "Max Time", 
            "Макс. час");
        add_translation(&mut translations, "col_success_rate", 
            "Success Rate", 
            "Успішність");
        add_translation(&mut translations, "col_median_time", 
            "Median", 
            "Медіана");
        add_translation(&mut translations, "col_std_dev", 
            "Std Dev", 
            "Стд. відхил.");
        add_translation(&mut translations, "col_throughput", 
            "Throughput", 
            "Пропускна зд.");
            
        // Decoding stats
        add_translation(&mut translations, "decoding_stats_title", 
            "Decoding Statistics", 
            "Статистика декодування");
        add_translation(&mut translations, "total_rows", 
            "Total rows:", 
            "Всього рядків:");
        add_translation(&mut translations, "successful_rows", 
            "Successful:", 
            "Успішних:");
        add_translation(&mut translations, "failed_rows", 
            "Failed:", 
            "Невдалих:");
        add_translation(&mut translations, "avg_iterations", 
            "Avg iterations:", 
            "Сер. ітерацій:");
        add_translation(&mut translations, "max_iter_hit", 
            "Hit max iterations:", 
            "Досягли ліміту:");

        // Parallel metrics  
        add_translation(&mut translations, "thread_count", 
            "Threads used:", 
            "Використано потоків:");
        add_translation(&mut translations, "parallel_efficiency", 
            "Efficiency:", 
            "Ефективність:");
        add_translation(&mut translations, "shares_per_sec", 
            "Shares/sec:", 
            "Часток/сек:");
            
        add_translation(&mut translations, "chart_title", 
            "Average Execution Time Chart", 
            "Графік середнього часу виконання");
        add_translation(&mut translations, "axis_time_ms", 
            "Time (ms)", 
            "Час (мс)");
        add_translation(&mut translations, "axis_parameters", 
            "Parameters", 
            "Параметри");
        add_translation(&mut translations, "impl_sequential", 
            "Sequential", 
            "Послідовно");
        add_translation(&mut translations, "impl_parallel", 
            "Parallel", 
            "Паралельно");
        add_translation(&mut translations, "legend_sequential", 
            "Sequential", 
            "Послідовна");
        add_translation(&mut translations, "legend_parallel", 
            "Parallel", 
            "Паралельна");
        add_translation(&mut translations, "chart_comparison_title", 
            "Execution Time Comparison", 
            "Порівняння часу виконання");
        add_translation(&mut translations, "speedup_info_title", 
            "Speedup Information", 
            "Інформація про прискорення");
        add_translation(&mut translations, "label_sequential", 
            "Sequential:", 
            "Послідовно:");
        add_translation(&mut translations, "label_parallel", 
            "Parallel:", 
            "Паралельно:");
        add_translation(&mut translations, "label_speedup", 
            "Speedup:", 
            "Прискорення:");
        add_translation(&mut translations, "speedup_percent_faster", 
            "% faster", 
            "% швидше");

        // Details tab
        add_translation(&mut translations, "setup_time_title", 
            "Setup Time", 
            "Час налаштування");
        add_translation(&mut translations, "deal_time_title", 
            "Deal Time", 
            "Час поділу на частки");
        add_translation(&mut translations, "reconstruct_time_title", 
            "Reconstruct Time", 
            "Час реконструкції секрету");

        // Phases tab
        add_translation(&mut translations, "deal_phases_title", 
            "Deal Process Phases", 
            "Фази процесу поділу на частки");
        add_translation(&mut translations, "reconstruct_phases_title", 
            "Reconstruct Process Phases", 
            "Фази процесу реконструкції секрету");
        add_translation(&mut translations, "col_phase", 
            "Phase", 
            "Фаза");
        add_translation(&mut translations, "col_percent_total", 
            "% of total", 
            "% від загального");

        // Phase distribution
        add_translation(&mut translations, "phase_distribution", 
            "Phase distribution:", 
            "Розподіл фаз:");

        // Enum labels - Info sizes
        add_translation(&mut translations, "info_size_k1024", 
            "1024 bits", 
            "1024 біт");
        add_translation(&mut translations, "info_size_k4096", 
            "4096 bits", 
            "4096 біт");
        add_translation(&mut translations, "info_size_k16384", 
            "16384 bits", 
            "16384 біт");

        // Enum labels - Rates
        add_translation(&mut translations, "rate_r1_2", 
            "1/2", 
            "1/2");
        add_translation(&mut translations, "rate_r2_3", 
            "2/3", 
            "2/3");
        add_translation(&mut translations, "rate_r4_5", 
            "4/5", 
            "4/5");

        // Enum labels - Decoders (user-friendly names)
        add_translation(&mut translations, "decoder_aminstarf32", 
            "Aminstar (f32)", 
            "Aminstar (f32)");
        add_translation(&mut translations, "decoder_aminstarf64", 
            "Aminstar (f64)", 
            "Aminstar (f64)");
        add_translation(&mut translations, "decoder_phif32", 
            "Phi (f32)", 
            "Phi (f32)");
        add_translation(&mut translations, "decoder_phif64", 
            "Phi (f64)", 
            "Phi (f64)");
        add_translation(&mut translations, "decoder_tanhf32", 
            "Tanh (f32)", 
            "Tanh (f32)");
        add_translation(&mut translations, "decoder_tanhf64", 
            "Tanh (f64)", 
            "Tanh (f64)");
        add_translation(&mut translations, "decoder_minstarappoxbonespartialhardlimit", 
            "MinStar Approx Bones Partial Hard Limit", 
            "MinStar Approx Bones Partial Hard Limit");
        add_translation(&mut translations, "decoder_minstarappoxbonespartialhardlimitmtdeg1clip", 
            "MinStar Approx Bones Partial Hard Limit MtDeg1Clip", 
            "MinStar Approx Bones Partial Hard Limit MtDeg1Clip");

        // Phases tab - expand/collapse all
        add_translation(&mut translations, "expand_all", 
            "Expand all", 
            "Розгорнути все");
        add_translation(&mut translations, "collapse_all", 
            "Collapse all", 
            "Згорнути все");

        Self {
            current_language: Language::Ukrainian, // Default language
            translations,
        }
    }
}

fn add_translation(
    translations: &mut HashMap<String, HashMap<Language, String>>,
    key: &str,
    english: &str,
    ukrainian: &str,
) {
    let mut lang_map = HashMap::new();
    lang_map.insert(Language::English, english.to_string());
    lang_map.insert(Language::Ukrainian, ukrainian.to_string());
    translations.insert(key.to_string(), lang_map);
}

impl Localization {
    pub fn get(&self, key: &str) -> &str {
        match self.translations.get(key) {
            Some(lang_map) => match lang_map.get(&self.current_language) {
                Some(text) => text,
                None => "[Translation missing]",
            },
            None => "[Unknown key]",
        }
    }

    pub fn set_language(&mut self, language: Language) {
        self.current_language = language;
    }

    #[allow(dead_code)]
    pub fn current_language(&self) -> &Language {
        &self.current_language
    }
}