pub struct Room {
    //真实房间号
    pub room_id: String,
    //主播user
    pub user: User,
    //主播公告
    pub content: String,
    //公告发布时间
    pub content_time: String,
    //不知道什么状态
    pub statue: String,
}


pub struct User{
    pub uid:String,
    pub username:String,
}