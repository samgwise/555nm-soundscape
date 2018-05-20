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
    fn diag() {
        let show_diag = false; // toggle to fail and print the following diag info
        let config = test_config();
        let today = local_today();
        println!("today {} ({:?})", from_timestamp(moment(&today) as i64), today);
        let start = next_start_time(&config, &today);
        println!("start {} ({:?})", from_timestamp(moment(&start) as i64), start);
        let end = next_end_time(&config, &start).unwrap();
        println!("end {} ({:?})", from_timestamp(moment(&end) as i64), end);
        println!("now {} ({:?})", from_timestamp(moment(&localtime()) as i64), localtime());
        println!("Is now in schedule? {}", is_in_schedule(&localtime(), &start, &end));
        assert!(!show_diag);
    }

    #[test]
    fn schedules() {
        let config = test_config();
        println!("now: {:?}", &epochsy::now());
        println!("localtime: {:?}", localtime());
        println!("11:45:26 <=> {:?}", &epochsy::hms(11, 45, 26));
        println!("midnight today <=> {:?}", epochsy::moment(&to_localtime(&epochsy::floor_to_days(&epochsy::now()))));

        println!("is_in_schedule_now currently? {:?}", is_in_schedule_now(&config, &localtime()));

        let start = next_start_time(&config, &localtime());
        assert!(start.moment > 0);

        // assert_eq!(from_timestamp(start).timestamp(), start);
        // let test_now = now();
        // assert_eq!(from_timestamp(test_now.timestamp()).timestamp(), test_now.timestamp());
        // println!("{:?} <=> {:?}", test_now, test_now.timestamp());

        let end_time_from_start = next_end_time(&config, &start);
        assert_ne!(end_time_from_start, None);
        let end_from_start = end_time_from_start.unwrap();
        let end_time = next_end_time(&config, &localtime());
        assert_ne!(end_time, None);
        let end = end_time.unwrap();

        // assert_eq!(to_localtime(&from_localtime(&start)).moment, start.moment);
        println!("working with start: {:?} and end: {:?}", start, end );
        println!("total interval in seconds: {:?}", epochsy::diff(&end, &start));

        // characteristic features
        assert!(moment(&start) < moment(&end_from_start));
        // assert expected duration
        assert_eq!(moment(&end_from_start) - moment(&start), 23400);

        // assert_eq!(start_local.hour(), 18);
        // assert_eq!(start_local.minute(), 30);
        // assert_eq!(start_local.second(), 0);
        //
        // assert_eq!(end_local.hour(), 1);
        // assert_eq!(end_local.minute(), 0);
        // assert_eq!(end_local.second(), 0);


        println!("is_in_schedule currently? {:?} ({:?} to {:?})", is_in_schedule(&localtime(), &start, &end), start, end);
        assert!(is_in_schedule(&start, &start, &end_from_start));
        assert!(is_in_schedule(&end_from_start, &start, &end_from_start));

        let before = epochsy::append(&local_today(), &epochsy::hms(15, 39, 0));
        println!("before: {} ({:?})", from_timestamp(moment(&before) as i64), before);
        assert!(before.moment < next_start_time(&config, &before).moment);
        assert!(next_start_time(&config, &before).moment < end_from_start.moment);

        let after = epochsy::append(&local_today(), &epochsy::hms(25, 31, 0));
        println!("after: {} ({:?})", from_timestamp(moment(&after) as i64), after);
        let during = epochsy::append(&start, &epochsy::hms(1, 0, 0));
        println!("during: {} ({:?})", from_timestamp(moment(&during) as i64), during);

        assert!(!is_in_schedule(&before, &start, &end));
        assert!(!is_in_schedule(&after, &start, &end));

        assert!(is_in_schedule_now(&config, &start));
        assert!(is_in_schedule_now(&config, &during));
        assert!(!is_in_schedule_now(&config, &before));
        assert!(!is_in_schedule_now(&config, &after));
        let before_end = epochsy::append(&local_today(), &epochsy::hms(20, 29, 0));
        println!("before_end: {} ({:?})", from_timestamp(moment(&before_end) as i64), before_end);
        assert!(is_in_schedule_now(&config, &before_end));

        // if start > end and now < end {
        //      Play
        // }
        //  else {
        //      Pause
        //  }

        // assert!(
        //     !is_in_schedule(
        //         &end.checked_add_signed(Duration::milliseconds(1000)).unwrap()
        //         , &start
        //         , &end
        //     )
        // );
    }
}
