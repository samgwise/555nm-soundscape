#[cfg(test)]
mod config_test {
    use config::*;
    use ::epochsy;

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
        println!("now: {:?}", &epochsy::now());
        println!("11:45:26 <=> {:?}", &epochsy::hms(11, 45, 26));
        println!("midnight today <=> {:?}", &epochsy::floor_to_days(&epochsy::now()));


        let start = next_start_time(&config, &epochsy::now());
        assert!(start.moment > 0);

        // assert_eq!(from_timestamp(start).timestamp(), start);
        // let test_now = now();
        // assert_eq!(from_timestamp(test_now.timestamp()).timestamp(), test_now.timestamp());
        // println!("{:?} <=> {:?}", test_now, test_now.timestamp());

        let end_time = next_end_time(&config, &start);
        assert_ne!(end_time, None);
        let end = end_time.unwrap();

        println!("working with start: {:?} and end: {:?}", start, end );
        println!("total interval in seconds: {:?}", epochsy::diff(&end, &start));

        // characteristic features
        assert!(start.moment < end.moment);
        // assert expected duration
        assert_eq!(end.moment - start.moment, 23400);

        // assert_eq!(start_local.hour(), 18);
        // assert_eq!(start_local.minute(), 30);
        // assert_eq!(start_local.second(), 0);
        //
        // assert_eq!(end_local.hour(), 1);
        // assert_eq!(end_local.minute(), 0);
        // assert_eq!(end_local.second(), 0);


        println!("is_in_schedule now? {:?}", is_in_schedule(&epochsy::now(), &start, &end));
        assert!(is_in_schedule(&start, &start, &end));
        assert!(is_in_schedule(&end, &start, &end));
        // assert!(
        //     !is_in_schedule(
        //         &end.checked_add_signed(Duration::milliseconds(1000)).unwrap()
        //         , &start
        //         , &end
        //     )
        // );
    }
}
