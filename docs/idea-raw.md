Tôi đang muốn thực hiện ý tưởng ước mơ của tôi về một game open world fantasy có các cơ chế phức tạp, siêu thực tế, siêu tự nhiên. Game chỉ cần đồ họa 2D kẻ ô vuông đơn giản thôi cũng được, góc nhìn top down, game là tập dữ liệu 3 chiều (x, y, z) . Game tài nguyên được generate địa hình thực tế ngẫu nhiên dự trên seeds, để đảm bảo thế giới rộng lớn x,y,z là số 64 bits.
Game có thể có nhiều worlds khác nhau có thể connect thông qua các cổng kết nối (có thể tạo thông qua công nghệ ma thuật, công nghệ hiện đại, hoặc ngẫu nhiên hiếm gặp do xung đột năng lượng của thế giới), mỗi worlds có một id nào đó, và các world có thể có vai trò khác nhau có kiểm soát.
VD:
- World 1: thế giới trái đất (nơi có con người, elf, người thú, orcs, ... có rất nhiều sinh vật sống động sống, chiến tranh, .. ở đây)
- World 2: Thế giới bóng đêm ( Nơi vị thần hỗn mang sinh sống, đóng vai trò như cái ác thuần túy xâm chiếm các thế giới )
- World 3: Thế giới của các vị thần, đây là thế giới của các vị thần (giống thần thoại hy lạp) các vị thần tính cách khác nhau (có thể chọn người ở world 1 trở thành thần, ..) Đại loại tôi muốn nó giống với mấy vị thần hi lạp, có vị thần bảo vệ, thỉnh thoảng lại dục vọng, ... Rồi có các vị thần đại diện cho các sự kiện tự nhiên trong thế giới world 1. (World 2 cũng thích chiến tranh world 3)
- Super Ultra World: Đây là nhà, nơi khởi đầu của vị thần tối cao, True God, các vị thần world 3 thật ra cũng chỉ là các sinh vật mạnh bất thường. Nhưng True God là một vị thần thật sự là Tôi owner của thế giới. Các thực thể biết đến True God hay Super Ultra World đều kính nể, kể cả world 2 hiếu chiến cũng không dám bước vào đây nếu không được sự cho phép của True God. 
Thế giới này như là một sandbox để True God thử nghiệm các thứ , hoặc làm gì đó tùy thích.

## Đồ họa
Tôi sẽ dùng vue + thư viện đồ họa để vẽ đồ họa đơn giản là các lưới với ô vuông, tương ứng với 1 vị trí trên bản đồ, và sẽ được đánh màu khác nhau tương ứng với loại vật chất khác nhau (VD: dung nham, đất, không khí, ...) map sẽ là không gian 3 chiều, góc nhìn vẫn là góc nhìn từ trên xuống, sẽ nhìn theo kiểu cắt lớp

## Các thực thể sống
Mỗi thực thể sống có trí tuệ sẽ được llm quản lý và nhập vai. Mỗi cá thể khi sinh ra sẽ có những một bản mô tả các tham số bằng yaml và có thể điều chỉnh bởi llm với các tham số cho phép
VD: Mô tả chi tiết ngoại hình, tham số sức khỏe, khả năng chịu đựng, .... sẽ ảnh hưởng tới việc tương tác với thế giới
Mỗi thực thể có tham số khác nhau thì có thể có khả năng bay hoặc không, khả năng dùng phép, tốc độ di chuyển, tầm nhìn, chỉ số thông minh, khả năng cảm nhận nguy hiểm, ....
Với 1 số tham số không bị khóa thì nhân vật có thể phát triển bản thân bằng cách luyện tập hoặc giảm đi khi không luyện tập, ...
Ngoài ra mỗi thực thể sống sẽ có một RAG dùng để lưu trữ ký ức độc lập với yaml
Các thực thể có thể hình thành 1 xã hội để hoạt động hợp tác, hoặc chiến tranh, tham nhũng ,....

## Các công nghệ, thông số, ma thuật
Các nền văn minh, công nghệ, phép thuật, ma thuật có nhiều cấp độ.
Các thực thế, sinh vật đều có các thông số khác nhau về trí tuệ, kiến thức đa dạng khác nhau về nhiều lĩnh vực, ...
Các thực thể có thể truyền dạy các kiến thức, ... cho các thực thể khác, khả năng truyền dạy, học tập của các thực thể cũng phụ thuộc vào thông số của thực thể đó.
Các nên văn minh hoặc kiến thức, skill points, tech points, magic points, ... khi tích lũy đến thời điểm nào đó các entity có khả năng chọn học hay research tìm kiếm một loại công nghệ, phép thuật nào đó để nâng cao khả năng của bản thân.
Có những phép thuật công nghệ rất phức tạp đòi hỏi các entity của một quốc gia, hay đa quốc gia hợp tác với nhau để tạo ra, triển khai.
VD: phép, công nghệ mở cổng đến thế giới khác siêu phức tạp. Các phép triệu hồi thần, thực thể hỗn mang, ....
Các công nghệ chiến tranh hiện đại, các phép hủy diệt diện rộng ...
Hệ thống này có thể hoạt động giống như các cây vậy, có các phụ thuộc tài nguyên, chỉ số, ...

Đây là ý tượng raw cần làm chi tiết và cụ thể hơn để thực tế nhất có thể, đa dạng thú vị

## Hệ thống quản lý thế giới ( Cánh tay phải của chúa ) gọi tắt là Yuu
Là một hệ thống dùng để quản lý, tạo ra các sự kiện tự nhiên, các sự kiện xã hội, ... để tạo ra thú vị và đa dạng của thế giới, thúc đẩy sự phát triển.
- Yuu chịu trách nghiệm tạo ra các loài theo yêu cầu của chúa (VD loài người, động vật, rồng, quái vật, ...)
- Yuu chịu trách nghiệm đưa các thông số ngẫu nhiên hay có chủ đích cho một cá thể, để tạo ra sự đa dạng và thú vị cho thế giới (policy)
- Yuu cũng chịu trách nghiệm cho việc tạo ra các quy tắc phép thuật, quy tắc vật lý bằng cách tạo ra các hàm số, vd quy tắc pháp luật, khi 2 đối tượng đánh nhau và cast phép, thì 2 đối tượng sẽ thực hiện call các function mà nó biết
...

## Chúa (Tôi)
Tờ giờ tên của tôi là True God
Sẽ có khả năng can thiệp vào mọi dữ liệu, mọi prompt. Cũng có thể giao tiếp với yuu để hỗ trợ làm việc. Ngoài ra thì chúa cũng có thể nhập vai vào 1 nhân vật để tham gia vào hoạt động thế giới

## Tối ưu tính toán
Hãy tối ưu mức độ request llm để đảm bảo tốc độ xử lý cũng như giảm thiểu mức gọi llm
VD xử lý tính cách, hành động các cá thể có thể xử lý theo batch vào 1 prompt?
VD các hoạt động của thế giới tồn tại dưới dạng policy thuật toán do các LLm sinh ra?