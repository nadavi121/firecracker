pub mod digest;
use digest::*;
use std::collections::{HashMap, HashSet};
use uuid::{Uuid, Version};
mod patterns;
use patterns::*;
mod path;
use path::*;

pub fn check_values_req(values: &HashSet<String>) -> ValueDescriptor {
    search_for_patterns(values.iter().collect())
}
pub fn check_values_res(values: &HashMap<String, u32>) -> ValueDescriptor {
    let mut split = Split::<String>::from_hashmap(values);
    split.filter();
    search_for_patterns(split.values.iter().collect())
}
trait MapDigest {
    fn create_map(&mut self);
    fn turn_hash(&self, paths: Vec<Path>) -> Vec<Endpoint>;
}
impl MapDigest for Digest {
    fn turn_hash(&self, paths: Vec<Path>) -> Vec<Endpoint> {
        let ep_hashes = &self.ep_hash;
        let mut eps = vec![];
        for ep_hash in ep_hashes {
            let pos = paths
                .iter()
                .position(|path| path.path_ext == ep_hash.path)
                .unwrap();
            eps.push(Endpoint::from_hash(ep_hash, paths[pos].clone()))
        }
        eps
    }
    fn create_map(&mut self) {
        let links_hash = &self.link_hash;
        let mut groups: Vec<Group> = vec![];
        let mut links = vec![];
        let mut endpoints = HashSet::new();
        for ep_s in links_hash.keys() {
            let total: u64 = links_hash.get(&ep_s).unwrap().values().sum();
            for (ep_t, amount) in links_hash.get(&ep_s).unwrap().iter() {
                if total / amount <= 10 {
                    links.push(GroupLink {
                        from: ep_s.clone(),
                        to: ep_t.clone(),
                        strength: *amount,
                    });
                    endpoints.insert(ep_s.clone());
                    endpoints.insert(ep_t.clone());
                }
            }
        }
        self.eps = endpoints.iter().cloned().collect();
        let mut used_eps = HashSet::new();
        while !links.is_empty() {
            let mut links_new = vec![];
            let mut group = Group::default();
            let mut group_eps = HashSet::new();
            for i in 0..links.len() {
                if i > 0 && group_eps.contains(&links[i].from) {
                    group.links.push(links[i].clone());
                    group_eps.insert(links[i].to.clone());
                    used_eps.insert(links[i].to.clone());
                } else if i == 0 {
                    group.links.push(links[0].clone());
                    group_eps.insert(links[0].from.clone());
                    group_eps.insert(links[0].to.clone());
                    used_eps.insert(links[i].to.clone());
                    used_eps.insert(links[i].from.clone());
                } else {
                    links_new.push(links[i].clone());
                }
            }
            group.endpoints = group_eps.iter().cloned().collect();
            groups.push(group);
            links = links_new;
        }
        for ep in self.eps.iter() {
            if !used_eps.contains(ep) {
                groups.push(Group {
                    endpoints: vec![ep.clone()],
                    links: vec![],
                });
            }
        }
        self.groups = groups;
    }
}
trait MapEp {
    fn from_hash(ep_hash: &EndpointHash, path: Path) -> Endpoint;
    fn get_headers(headers: &HashMap<String, HashMap<String, u32>>) -> Vec<Header>;
    fn get_req_res_payloads(hash: &EndpointHash) -> RRPayload;
}

impl MapEp for Endpoint {
    fn get_headers(headers: &HashMap<String, HashMap<String, u32>>) -> Vec<Header> {
        //let mut total:u64 = 0;
        let mut req_headers = vec![];
        for header in headers.keys() {
            //total+= headers.get(header).unwrap().values().sum();
            let mut g = 0;
            let mut v = String::new();
            for (val, amount) in headers.get(header).unwrap().iter() {
                if *amount > g {
                    g = *amount;
                    v = val.clone();
                }
            }
            req_headers.push(Header {
                name: header.clone(),
                value: v,
            });
        }
        req_headers
    }
    fn get_req_res_payloads(hash: &EndpointHash) -> RRPayload {
        let mut req_payload_params = vec![];
        for (param, payloads) in hash.queries.reqp_map.iter() {
            let value = check_values_req(payloads);
            req_payload_params.push(ParamDescriptor {
                from: QuePay::Query,
                name: param.to_string(),
                value,
            });
        }
        for (param, payloads) in hash.status_payloads.reqp_map.iter() {
            println!("{}", hash.path);
            println!("{}\t\t{:?}", param, payloads);
            let value = check_values_req(payloads);
            req_payload_params.push(ParamDescriptor {
                from: QuePay::Payload,
                name: param.to_string(),
                value,
            });
        }
        let req_payload = PayloadDescriptor {
            params: req_payload_params,
        };

        let mut res_payload_params = vec![];
        let mut statuses = hash.queries.status_map.clone();
        for (param, payloads) in hash.queries.resp_map.iter() {
            let value = check_values_res(payloads);
            res_payload_params.push(ParamDescriptor {
                from: QuePay::Response,
                name: param.to_string(),
                value,
            });
        }
        statuses.extend(&hash.status_payloads.status_map);
        for (param, payloads) in hash.status_payloads.resp_map.iter() {
            let value = check_values_res(payloads);
            res_payload_params.push(ParamDescriptor {
                from: QuePay::Response,
                name: param.to_string(),
                value,
            });
        }
        let res_payload = PayloadDescriptor {
            params: res_payload_params,
        };
        RRPayload {
            status: Split::from_hashmap(&statuses),
            req_payload,
            res_payload,
        }
    }
    fn from_hash(ep_hash: &EndpointHash, path: Path) -> Endpoint {
        let common_req_headers = HeaderMap::new(Self::get_headers(&ep_hash.req_headers));
        let common_res_headers = HeaderMap::new(Self::get_headers(&ep_hash.res_headers));
        Endpoint {
            common_req_headers,
            common_res_headers,
            path,
            methods: Split::from_hashmap(&ep_hash.methods),
            payload_delivery_methods: Split::from_hashmap(&ep_hash.dm),
            req_res_payloads: Self::get_req_res_payloads(ep_hash),
        }
    }
}
pub trait MapLoad {
    fn load_session(&mut self, _session: Session);
    fn load_vec_session(&mut self, _sessions: Vec<Session>);
    fn load_req_res(&mut self, _req_res: ReqRes);
    fn load_vec_req_res(&mut self, _req_reses: Vec<ReqRes>);
}
impl MapLoad for Digest {
    fn load_vec_session(&mut self, sessions: Vec<Session>) {
        let mut paths = vec![];
        for s in sessions.iter() {
            let pts: Vec<String> = s
                .req_res
                .iter()
                .map(|rr| {
                    //if rr.status != 404{
                    let end_bytes = rr.path.find('?').unwrap_or_else(|| rr.path.len());
                    rr.path[..end_bytes].to_string()
                    //}else{
                    //  String::new()
                    //}
                })
                .collect();
            paths.extend(pts)
        }
        let paths1 = first_cycle(paths);
        let paths = second_cycle(paths1);
        let mut paths_hash = HashMap::new();
        for session in sessions.iter() {
            if session.req_res.is_empty() {
                continue;
            }
            for i in 0..session.req_res.len() {
                let ep_path_ext = to_ext(session.req_res[i].path.clone());
                if let Some(pos) = self
                    .ep_hash
                    .iter_mut()
                    .position(|ep_h| ep_h.path.clone() == ep_path_ext)
                {
                    let a = paths_hash.entry(ep_path_ext.clone()).or_insert(0);
                    *a += 1;
                    self.ep_hash[pos].load(&session.req_res[i]);
                } else {
                    let mut ep1 = EndpointHash::new(ep_path_ext.clone());
                    let a = paths_hash.entry(ep_path_ext.clone()).or_insert(0);
                    *a += 1;
                    ep1.load(&session.req_res[i]);
                    self.ep_hash.push(ep1);
                }
            }
        }
        let eps = self.turn_hash(paths);
        let mut links = vec![];
        for session in sessions {
            for i in 0..(session.req_res.len() - 1) {
                for j in (i + 1)..session.req_res.len() {
                    let path1 = to_ext(session.req_res[i].path.clone());
                    //HAS TO BE ONE!!!!!!
                    let pos1 = eps.iter().position(|ep| ep.path.path_ext == path1).unwrap();
                    let path2 = to_ext(session.req_res[j].path.clone());
                    //HAS TO BE ONE!!!!!!
                    let pos2 = eps.iter().position(|ep| ep.path.path_ext == path2).unwrap();
                    links.push(Link {
                        from: eps[pos1].clone(),
                        to: eps[pos2].clone(),
                    });
                }
            }
        }
        self.link_hash.load_data(links);
        self.create_map();
        self.path_hash = paths_hash;
    }
    fn load_session(&mut self, _session: Session) {}
    /*
    fn load_session(&mut self, session: Session) {
        if session.req_res.is_empty(){
            return;
        }
        for i in 0..(session.req_res.len() - 1) {
            let mut found = false;
            for ep_hash in &mut self.ep_hash {
                if ep_hash.path == session.req_res[i].path {
                    ep_hash.load(&session.req_res[i]);
                    found = true;
                }
            }
            if !found  {
                let mut ep1 = EndpointHash::new(session.req_res[i].path.clone());
                ep1.load(&session.req_res[i]);
                self.ep_hash.push(ep1);
            }
        }
        let eps = self.turn_hash();
        let mut links = vec![];
        for i in 0..(session.req_res.len() - 1) {
            //should be one!!!
            let index = eps
                .iter()
                .position(|ep| ep.path.path_ext == to_ext(session.req_res[i].path))
                .unwrap();
            let from = eps[index].clone();
            for j in (i + 1)..(session.req_res.len() - 1) {
                let index = eps
                    .iter()
                    .position(|ep| ep.path.path_ext == to_ext(session.req_res[j].path))
                    .unwrap();
                let to = eps[index].clone();
                links.push(Link {
                    from: from.clone(),
                    to,
                });
            }
        }
        self.link_hash.load_data(links);
        self.create_map();
    }*/
    fn load_req_res(&mut self, _req_res: ReqRes) {}
    fn load_vec_req_res(&mut self, _req_reses: Vec<ReqRes>) {}
}
