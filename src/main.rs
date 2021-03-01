use std::{collections::HashMap, sync::Arc};
use std::iter::{self, Iterator};
use num_traits::cast::FromPrimitive;
use serde::{Serialize, Deserialize};
use chrono::{Datelike, Weekday, naive::NaiveDate};
use handlebars::Handlebars;
use warp::Filter;

const TEMPLATE_INDEX: &str = "index";


// данная структура безопасно сериализуется и копируется
#[derive(Serialize, Deserialize, Debug, Clone)]        
// структура хранение данных о дне календаря
struct Day {                                           
    // текст в ячейке календаря
    txt: String,                                       
    // является ли красным днём
    red: bool                                          
}

// объявление дружественных функций для структуры Day
impl Day {
    // создаёт пустую ячуйку календаря
    fn empty() -> Self {
        return Day { txt: "".to_string(), red: false };
    }
}
// обьявляет возможность создания Day из структуры NaiveDate
// день календаря красный, если в NaiveDate указан как сб или вс
impl From<NaiveDate> for Day {
    fn from(date: NaiveDate) -> Self {
        return Day { 
            txt: date.day().to_string(), 
            red: date.weekday() == Weekday::Sun || date.weekday() == Weekday::Sat,
        };
    }
}

// структура для хранения месяца
#[derive(Serialize, Deserialize, Debug)]
struct Month {
    // название месяца в заголовку
    name: String,
    // таблица дней
    days: Vec<Vec<Day>>
}

impl Month {
    // конструктор календарного месяца
    // принимает номер месяца и год
    fn new(order: u32, year: i32) -> Option<Self> {
        // найти первый день месяца в библиотеке chrono
        let date = NaiveDate::from_ymd(year, order, 1);
        // найти месяц по номеру
        let month = chrono::Month::from_u32(date.month())?;
        // определить день недели первого дня месяцв
        let weekday = date.weekday().number_from_monday();
        // определить следующий месяц
        let next_month = month.succ().number_from_month();
        // определить, находится ли следующий месяц в следующем году
        let next_months_year = if next_month > date.month() {
            date.year() 
        } else {
            date.year() + 1 
        };
        // найти первый ден следующего месяца
        let next_month_date = NaiveDate::from_ymd(next_months_year, next_month, 1);
        // теперь, определить количество дней в месяце
        let days_in_month = next_month_date
            .signed_duration_since(date)
            .num_days();

        // сопоставить перечисление месяцев с русским представлением
        let month_rus = match month {
            chrono::Month::January  => "Январь",
            chrono::Month::February => "Февраль",
            chrono::Month::March    => "Март",
            chrono::Month::April    => "Апрель",
            chrono::Month::May      => "Май",
            chrono::Month::June     => "Июнь",
            chrono::Month::July     => "Июль",
            chrono::Month::August   => "Август",
            chrono::Month::September=> "Сентябрь",
            chrono::Month::October  => "Октябрь",
            chrono::Month::November => "Ноябрь",
            chrono::Month::December => "Декабрь"
        };

        // название месяца - это месяц + год
        let month_name = format!("{} {}", month_rus, year);
        
        // создание таблицы календарных дней
        // повторять None (аналог null)...
        let rows = iter::repeat(None)
            // ...чтобы сдвинуть первый день недели
            .take((weekday - 1) as usize)
            // и добавить определённое количество дней от 1 до N
            .chain((1..=days_in_month).map(|e| Some(e)))
            // превратить эти дни в даты из chrono
            .map(|o| o.map(|day| NaiveDate::from_ymd(year, order, day as u32)))
            // с помощью конструктора заменить даты на ячейки (заменяя null на пустые)
            .map(|o| o.map_or_else(Day::empty, Day::from))
            .collect::<Vec<Day>>()
            // разделить на кусочки по 7 дней
            .chunks(7)
            .map(|c| c.to_vec())
            // собрать таблицу
            .collect();

        return Some(Month { name: month_name, days: rows });
    }
    // для создания из строки типа `2021-03`
    fn from_date_notation(date_notation: &String) -> Option<Self> {
        let mut iter = date_notation
                // разделить по символу `-`
                .split("-")
                // конвертировать в числа
                .map(|s| s.parse::<i64>().ok());
        // первое число - год
        let year = iter.next()?? as i32;
        // второе число - месяц
        let month = iter.next()?? as u32;
        // использовать стандартный конструктор
        return Self::new(month,year);
    }
}

#[tokio::main]
async fn main() {
    // создать объект шаблонного движка
    let mut handlebars = Handlebars::new();                

    // зареестрировать темплейт из файла
    handlebars                                             
        .register_template_file(TEMPLATE_INDEX, "templates/index.hbs")
        // вывести ошибку если файл не найден
        .expect("Could not load template file index.hbs"); 

    let handlebars = Arc::new(handlebars);                 

    // объявить путь GET
    let route = warp::get()                                
        .and(warp::path::end())                            
        // получить список параметров в виде ассоциативного массива
        .and(warp::query::<HashMap<String, String>>())
        .map(move |p: HashMap<String,String>| { 
            // получить значение параметра запроса
            let month = p.get("month");
            let month = match month { 
                // если параметр доступен - создать объект календарного месяца
                Some(month) => Month::from_date_notation(month),
                None => None
            // если месяц не доступен - создать месяц по умолчанию
            // Алексей Метлицкий; дата рождения - 21.03.2000
            }.unwrap_or(Month::new(3,2017).unwrap());
            let rendered = handlebars
                // обработка шаблона
                .render(TEMPLATE_INDEX, &month)            
                // если произошла ошибка - вывести её вместо результата
                .unwrap_or_else(|e| e.to_string());        
            // обернуть в http ответ
            return warp::reply::html(rendered);            

        });

    // запустить сервер
    warp::serve(route).run(([0,0,0,0], 8080)).await;

}

