use trust_dns_server::authority::MessageRequest;
use trust_dns::op::Message;
use trust_dns::op::Query;

pub fn message_request_to_message(mr: &MessageRequest)-> Message {
    let mut m = Message::new();
    m.set_id(mr.id());
    m.set_message_type(mr.message_type());
    m.set_op_code(mr.op_code());
    m.set_recursion_desired(mr.recursion_desired());
    let qs: Vec<Query> = mr.queries().iter().map(|q| q.original().clone()).collect();
    m.add_queries(qs);
    m.add_answers(mr.answers().to_vec());
    m.add_name_servers(mr.name_servers().to_vec());
    m.insert_additionals(mr.additionals().to_vec());
    if let Some(e) = mr.edns() {
        m.set_edns(e.clone());
    }
    m
}