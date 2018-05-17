#[cfg(test)]
mod config_test {
    use config::*;

    fn test_config() -> Soundscape {
        Soundscape {
            listen_addr:            Address { host: "127.0.0.1".to_string(), port: 4000 },
            subscribers:            vec![ Address { host: "127.0.0.1".to_string(), port: 4000 } ],
            scenes:                 vec![],
            metro_step_ms:          10,
            voice_limit:            16,
            default_level:          1.0,
            background_scene:       None,
            speaker_positions:      Speakers { positions: vec![] },
            ignore_extra_speakers:  Some (true),
            is_fallback_slave:      None,
            daily_schedule:         Some (DailySchedule { start: "18:30:00".to_string(), end: "01:00:00".to_string() }),
        }
    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn schedules() {
        let config = test_config();
        let start_time = next_start_time(&config);
        assert_ne!(start_time, None);
        let end_time = finish_time(&config, &start_time.unwrap());
        assert_ne!(end_time, None);
        print!("working with start: {:?} and end: {:?}", start_time, end_time );
        assert!(start_time.unwrap() < end_time.unwrap());
    }
}
