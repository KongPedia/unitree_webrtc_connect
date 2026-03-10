use pyo3::prelude::*;
use pyo3::types::PyDict;

#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebRTCConnectionMethod {
    LocalAP = 1,
    LocalSTA = 2,
    Remote = 3,
}

#[pyclass]
#[allow(non_camel_case_types)]
pub struct VUI_COLOR;

#[pymethods]
impl VUI_COLOR {
    #[classattr]
    const WHITE: &'static str = "white";
    #[classattr]
    const RED: &'static str = "red";
    #[classattr]
    const YELLOW: &'static str = "yellow";
    #[classattr]
    const BLUE: &'static str = "blue";
    #[classattr]
    const GREEN: &'static str = "green";
    #[classattr]
    const CYAN: &'static str = "cyan";
    #[classattr]
    const PURPLE: &'static str = "purple";
}

pub fn register_constants(py: Python, m: &Bound<'_, pyo3::types::PyModule>) -> PyResult<()> {
    m.add_class::<WebRTCConnectionMethod>()?;
    m.add_class::<VUI_COLOR>()?;

    let data_channel_type = PyDict::new_bound(py);
    data_channel_type.set_item("VALIDATION", "validation")?;
    data_channel_type.set_item("SUBSCRIBE", "subscribe")?;
    data_channel_type.set_item("UNSUBSCRIBE", "unsubscribe")?;
    data_channel_type.set_item("MSG", "msg")?;
    data_channel_type.set_item("REQUEST", "req")?;
    data_channel_type.set_item("RESPONSE", "res")?;
    data_channel_type.set_item("VID", "vid")?;
    data_channel_type.set_item("AUD", "aud")?;
    data_channel_type.set_item("ERR", "err")?;
    data_channel_type.set_item("HEARTBEAT", "heartbeat")?;
    data_channel_type.set_item("RTC_INNER_REQ", "rtc_inner_req")?;
    data_channel_type.set_item("RTC_REPORT", "rtc_report")?;
    data_channel_type.set_item("ADD_ERROR", "add_error")?;
    data_channel_type.set_item("RM_ERROR", "rm_error")?;
    data_channel_type.set_item("ERRORS", "errors")?;
    m.add("DATA_CHANNEL_TYPE", data_channel_type)?;

    let app_error_messages = PyDict::new_bound(py);
    app_error_messages.set_item("app_error_code_100_1", "DDS message timeout")?;
    app_error_messages.set_item("app_error_code_100_10", "Battery communication error")?;
    app_error_messages.set_item("app_error_code_100_2", "Distribution switch abnormal")?;
    app_error_messages.set_item(
        "app_error_code_100_20",
        "Abnormal mote control communication",
    )?;
    app_error_messages.set_item("app_error_code_100_40", "MCU communication error")?;
    app_error_messages.set_item("app_error_code_100_80", "Motor communication error")?;
    app_error_messages.set_item("app_error_code_200_1", "Rear left fan jammed")?;
    app_error_messages.set_item("app_error_code_200_2", "Rear right fan jammed")?;
    app_error_messages.set_item("app_error_code_200_4", "Front fan jammed")?;
    app_error_messages.set_item("app_error_code_300_1", "Overcurrent")?;
    app_error_messages.set_item("app_error_code_300_10", "Winding overheating")?;
    app_error_messages.set_item("app_error_code_300_100", "Motor communication interruption")?;
    app_error_messages.set_item("app_error_code_300_2", "Overvoltage")?;
    app_error_messages.set_item("app_error_code_300_20", "Encoder abnormal")?;
    app_error_messages.set_item("app_error_code_300_4", "Driver overheating")?;
    app_error_messages.set_item("app_error_code_300_8", "Generatrix undervoltage")?;
    app_error_messages.set_item("app_error_code_400_1", "Motor rotate speed abnormal")?;
    app_error_messages.set_item("app_error_code_400_10", "Abnormal dirt index")?;
    app_error_messages.set_item("app_error_code_400_2", "PointCloud data abnormal")?;
    app_error_messages.set_item("app_error_code_400_4", "Serial port data abnormal")?;
    app_error_messages.set_item("app_error_code_500_1", "UWB serial port open abnormal")?;
    app_error_messages.set_item(
        "app_error_code_500_2",
        "Robot dog information retrieval abnormal",
    )?;
    app_error_messages.set_item("app_error_code_600_4", "Overheating software protection")?;
    app_error_messages.set_item("app_error_code_600_8", "Low battery software protection")?;
    app_error_messages.set_item("app_error_source_100", "Communication firmware malfunction")?;
    app_error_messages.set_item("app_error_source_200", "Communication firmware malfunction")?;
    app_error_messages.set_item("app_error_source_300", "Motor malfunction")?;
    app_error_messages.set_item("app_error_source_400", "Radar malfunction")?;
    app_error_messages.set_item("app_error_source_500", "UWB malfunction")?;
    app_error_messages.set_item("app_error_source_600", "Motion Control")?;
    app_error_messages.set_item(
        "app_error_wheel_300_100",
        "Motor Communication Interruption",
    )?;
    app_error_messages.set_item("app_error_wheel_300_40", "Calibration Data Abnormality")?;
    app_error_messages.set_item("app_error_wheel_300_80", "Abnormal Reset")?;
    m.add("app_error_messages", app_error_messages)?;

    let rtc_topic = PyDict::new_bound(py);
    rtc_topic.set_item("LOW_STATE", "rt/lf/lowstate")?;
    rtc_topic.set_item("MULTIPLE_STATE", "rt/multiplestate")?;
    rtc_topic.set_item("FRONT_PHOTO_REQ", "rt/api/videohub/request")?;
    rtc_topic.set_item("ULIDAR_SWITCH", "rt/utlidar/switch")?;
    rtc_topic.set_item("ULIDAR", "rt/utlidar/voxel_map")?;
    rtc_topic.set_item("ULIDAR_ARRAY", "rt/utlidar/voxel_map_compressed")?;
    rtc_topic.set_item("ULIDAR_STATE", "rt/utlidar/lidar_state")?;
    rtc_topic.set_item("ROBOTODOM", "rt/utlidar/robot_pose")?;
    rtc_topic.set_item("UWB_REQ", "rt/api/uwbswitch/request")?;
    rtc_topic.set_item("UWB_STATE", "rt/uwbstate")?;
    rtc_topic.set_item("LOW_CMD", "rt/lowcmd")?;
    rtc_topic.set_item("WIRELESS_CONTROLLER", "rt/wirelesscontroller")?;
    rtc_topic.set_item("SPORT_MOD", "rt/api/sport/request")?;
    rtc_topic.set_item("SPORT_MOD_STATE", "rt/sportmodestate")?;
    rtc_topic.set_item("LF_SPORT_MOD_STATE", "rt/lf/sportmodestate")?;
    rtc_topic.set_item("BASH_REQ", "rt/api/bashrunner/request")?;
    rtc_topic.set_item("SELF_TEST", "rt/selftest")?;
    rtc_topic.set_item("GRID_MAP", "rt/mapping/grid_map")?;
    rtc_topic.set_item("SERVICE_STATE", "rt/servicestate")?;
    rtc_topic.set_item("GPT_FEEDBACK", "rt/gptflowfeedback")?;
    rtc_topic.set_item("VUI", "rt/api/vui/request")?;
    rtc_topic.set_item("OBSTACLES_AVOID", "rt/api/obstacles_avoid/request")?;
    rtc_topic.set_item("SLAM_QT_COMMAND", "rt/qt_command")?;
    rtc_topic.set_item("SLAM_ADD_NODE", "rt/qt_add_node")?;
    rtc_topic.set_item("SLAM_ADD_EDGE", "rt/qt_add_edge")?;
    rtc_topic.set_item("SLAM_QT_NOTICE", "rt/qt_notice")?;
    rtc_topic.set_item("SLAM_PC_TO_IMAGE_LOCAL", "rt/pctoimage_local")?;
    rtc_topic.set_item("SLAM_ODOMETRY", "rt/lio_sam_ros2/mapping/odometry")?;
    rtc_topic.set_item("ARM_COMMAND", "rt/arm_Command")?;
    rtc_topic.set_item("ARM_FEEDBACK", "rt/arm_Feedback")?;
    rtc_topic.set_item("AUDIO_HUB_REQ", "rt/api/audiohub/request")?;
    rtc_topic.set_item("AUDIO_HUB_PLAY_STATE", "rt/audiohub/player/state")?;
    rtc_topic.set_item("GAS_SENSOR", "rt/gas_sensor")?;
    rtc_topic.set_item("GAS_SENSOR_REQ", "rt/api/gas_sensor/request")?;
    rtc_topic.set_item("LIDAR_MAPPING_CMD", "rt/uslam/client_command")?;
    rtc_topic.set_item(
        "LIDAR_MAPPING_CLOUD_POINT",
        "rt/uslam/frontend/cloud_world_ds",
    )?;
    rtc_topic.set_item("LIDAR_MAPPING_ODOM", "rt/uslam/frontend/odom")?;
    rtc_topic.set_item("LIDAR_MAPPING_PCD_FILE", "rt/uslam/cloud_map")?;
    rtc_topic.set_item("LIDAR_MAPPING_SERVER_LOG", "rt/uslam/server_log")?;
    rtc_topic.set_item("LIDAR_LOCALIZATION_ODOM", "rt/uslam/localization/odom")?;
    rtc_topic.set_item(
        "LIDAR_NAVIGATION_GLOBAL_PATH",
        "rt/uslam/navigation/global_path",
    )?;
    rtc_topic.set_item(
        "LIDAR_LOCALIZATION_CLOUD_POINT",
        "rt/uslam/localization/cloud_world",
    )?;
    rtc_topic.set_item(
        "PROGRAMMING_ACTUATOR_CMD",
        "rt/programming_actuator/command",
    )?;
    rtc_topic.set_item("ASSISTANT_RECORDER", "rt/api/assistant_recorder/request")?;
    rtc_topic.set_item("MOTION_SWITCHER", "rt/api/motion_switcher/request")?;
    m.add("RTC_TOPIC", rtc_topic)?;

    let sport_cmd = PyDict::new_bound(py);
    sport_cmd.set_item("Damp", 1001)?;
    sport_cmd.set_item("BalanceStand", 1002)?;
    sport_cmd.set_item("StopMove", 1003)?;
    sport_cmd.set_item("StandUp", 1004)?;
    sport_cmd.set_item("StandDown", 1005)?;
    sport_cmd.set_item("RecoveryStand", 1006)?;
    sport_cmd.set_item("Euler", 1007)?;
    sport_cmd.set_item("Move", 1008)?;
    sport_cmd.set_item("Sit", 1009)?;
    sport_cmd.set_item("RiseSit", 1010)?;
    sport_cmd.set_item("SwitchGait", 1011)?;
    sport_cmd.set_item("Trigger", 1012)?;
    sport_cmd.set_item("BodyHeight", 1013)?;
    sport_cmd.set_item("FootRaiseHeight", 1014)?;
    sport_cmd.set_item("SpeedLevel", 1015)?;
    sport_cmd.set_item("Hello", 1016)?;
    sport_cmd.set_item("Stretch", 1017)?;
    sport_cmd.set_item("TrajectoryFollow", 1018)?;
    sport_cmd.set_item("ContinuousGait", 1019)?;
    sport_cmd.set_item("Content", 1020)?;
    sport_cmd.set_item("Wallow", 1021)?;
    sport_cmd.set_item("Dance1", 1022)?;
    sport_cmd.set_item("Dance2", 1023)?;
    sport_cmd.set_item("GetBodyHeight", 1024)?;
    sport_cmd.set_item("GetFootRaiseHeight", 1025)?;
    sport_cmd.set_item("GetSpeedLevel", 1026)?;
    sport_cmd.set_item("SwitchJoystick", 1027)?;
    sport_cmd.set_item("Pose", 1028)?;
    sport_cmd.set_item("Scrape", 1029)?;
    sport_cmd.set_item("FrontFlip", 1030)?;
    sport_cmd.set_item("LeftFlip", 1042)?;
    sport_cmd.set_item("RightFlip", 1043)?;
    sport_cmd.set_item("BackFlip", 1044)?;
    sport_cmd.set_item("FrontJump", 1031)?;
    sport_cmd.set_item("FrontPounce", 1032)?;
    sport_cmd.set_item("WiggleHips", 1033)?;
    sport_cmd.set_item("GetState", 1034)?;
    sport_cmd.set_item("EconomicGait", 1035)?;
    sport_cmd.set_item("LeadFollow", 1045)?;
    sport_cmd.set_item("FingerHeart", 1036)?;
    sport_cmd.set_item("Bound", 1304)?;
    sport_cmd.set_item("MoonWalk", 1305)?;
    sport_cmd.set_item("OnesidedStep", 1303)?;
    sport_cmd.set_item("CrossStep", 1302)?;
    sport_cmd.set_item("Handstand", 1301)?;
    sport_cmd.set_item("StandOut", 1039)?;
    sport_cmd.set_item("FreeWalk", 1045)?;
    sport_cmd.set_item("Standup", 1050)?;
    sport_cmd.set_item("CrossWalk", 1051)?;
    m.add("SPORT_CMD", sport_cmd)?;

    let audio_api = PyDict::new_bound(py);
    audio_api.set_item("GET_AUDIO_LIST", 1001)?;
    audio_api.set_item("SELECT_START_PLAY", 1002)?;
    audio_api.set_item("PAUSE", 1003)?;
    audio_api.set_item("UNSUSPEND", 1004)?;
    audio_api.set_item("SELECT_PREV_START_PLAY", 1005)?;
    audio_api.set_item("SELECT_NEXT_START_PLAY", 1006)?;
    audio_api.set_item("SET_PLAY_MODE", 1007)?;
    audio_api.set_item("SELECT_RENAME", 1008)?;
    audio_api.set_item("SELECT_DELETE", 1009)?;
    audio_api.set_item("GET_PLAY_MODE", 1010)?;
    audio_api.set_item("UPLOAD_AUDIO_FILE", 2001)?;
    audio_api.set_item("PLAY_START_OBSTACLE_AVOIDANCE", 3001)?;
    audio_api.set_item("PLAY_EXIT_OBSTACLE_AVOIDANCE", 3002)?;
    audio_api.set_item("PLAY_START_COMPANION_MODE", 3003)?;
    audio_api.set_item("PLAY_EXIT_COMPANION_MODE", 3004)?;
    audio_api.set_item("ENTER_MEGAPHONE", 4001)?;
    audio_api.set_item("EXIT_MEGAPHONE", 4002)?;
    audio_api.set_item("UPLOAD_MEGAPHONE", 4003)?;
    audio_api.set_item("INTERNAL_LONG_CORPUS_SELECT_TO_PLAY", 5001)?;
    audio_api.set_item("INTERNAL_LONG_CORPUS_PLAYBACK_COMPLETED", 5002)?;
    audio_api.set_item("INTERNAL_LONG_CORPUS_STOP_PLAYING", 5003)?;
    m.add("AUDIO_API", audio_api)?;

    Ok(())
}
