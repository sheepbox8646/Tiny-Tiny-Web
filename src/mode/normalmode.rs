/* Tiny Tiny Web
 * Copyright (C) 2024 Plasma (https://github.com/duoduo70/Tiny-Tiny-Web/).
 *
 * You should have received a copy of the GNU General Public License
 * along with this program;
 * if not, see <https://www.gnu.org/licenses/>.
 */

use std::{process::exit, sync::atomic::Ordering};

use super::utils::*;
use crate::drop::log::LogLevel::*;
use crate::i18n::LOG;
use crate::macros::*;
use crate::{ThreadPool, GLOBAL_CONFIG, THREADS_NUM};

pub fn start() -> ! {
    let config = config_init();

    log!(Info, LOG[15]);

    let listener = listener_init(config);

    let mut threadpool = ThreadPool::new();

    let threads_num = THREADS_NUM.load(Ordering::Relaxed);

    for stream in listener.incoming() {
        match stream {
            Ok(req) => {
                threadpool.add(threads_num.try_into().unwrap(), || {
                    handle_connection(req, unsafe { &GLOBAL_CONFIG.clone().unwrap().clone() })
                });
            }
            Err(_) => continue, // TODO: add log
        }
    }
    exit(0);
}
